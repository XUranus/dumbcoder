#!/usr/bin/env python3
"""
Dumbcoder Agent Performance Evaluation Script
==============================================

Environment variables:
  DUMBCODER_BIN          Path to dumbcoder binary (default: target/release/dumbcoder)
  DUMBCODER_WORK_DIR     Test project directory (default: /opt/workspace/test-project)
  EVAL_BASE_URL          Eval LLM base URL (required)
  EVAL_API_KEY           Eval LLM API key (required)
  EVAL_MODEL             Eval LLM model name (default: mimo-v2.5-pro)
  EVAL_REQUEST_INTERVAL  Seconds between eval requests (default: 3.0)
  TEST_FILTER            Filter test cases by ID prefix (e.g., "ask" to run only ask tests)
"""

import json
import subprocess
import time
import sys
import os
import urllib.request
import urllib.error
from datetime import datetime

# ── Config from env ──

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DUMBCODER_BIN = os.environ.get(
    "DUMBCODER_BIN",
    os.path.join(SCRIPT_DIR, "../../target/release/dumbcoder"),
)
WORK_DIR = os.environ.get("DUMBCODER_WORK_DIR", "/opt/workspace/test-project")

EVAL_BASE_URL = os.environ.get("EVAL_BASE_URL", "")
EVAL_API_KEY = os.environ.get("EVAL_API_KEY", "")
EVAL_MODEL = os.environ.get("EVAL_MODEL", "mimo-v2.5-pro")
REQUEST_INTERVAL = float(os.environ.get("EVAL_REQUEST_INTERVAL", "3.0"))
MAX_RETRIES = 3
RETRY_BACKOFF = 5
TEST_FILTER = os.environ.get("TEST_FILTER", "")

# ── Test Cases ──

TEST_CASES = [
    {
        "id": "ask_easy_1",
        "command": "ask",
        "args": ["Where is the user authentication logic?"],
        "difficulty": "easy",
        "category": "ask",
        "eval_criteria": "Should mention src/services/auth_service.py and src/models/user.py. Should describe the login flow including password verification and session token generation.",
    },
    {
        "id": "ask_medium_1",
        "command": "ask",
        "args": ["How does the order cancellation flow work? What permissions are needed?"],
        "difficulty": "medium",
        "category": "ask",
        "eval_criteria": "Should explain cancel_order method in order_service.py, mention permission checks (owner or admin), reference OrderStatus state machine, describe transition to CANCELLED.",
    },
    {
        "id": "ask_hard_1",
        "command": "ask",
        "args": ["Explain the rate limiting mechanism in the auth service. How does it interact with the login flow?"],
        "difficulty": "hard",
        "category": "ask",
        "eval_criteria": "Should describe IP-based rate limiting in _is_rate_limited/_record_request, the 30 req/min limit, sliding window approach, and how it integrates into login.",
    },
    {
        "id": "explain_easy_1",
        "command": "explain",
        "args": ["src/utils/validators.py"],
        "difficulty": "easy",
        "category": "explain",
        "eval_criteria": "Should describe validate_email (regex pattern), validate_password_strength (scoring system), sanitize_input.",
    },
    {
        "id": "explain_symbol_1",
        "command": "explain",
        "args": ["src/models/order.py", "--symbol", "Order"],
        "difficulty": "medium",
        "category": "explain",
        "eval_criteria": "Should explain Order dataclass, properties (total_amount, item_count), can_transition_to state machine, transition_to method.",
    },
    {
        "id": "search_easy_1",
        "command": "search",
        "args": ["password validation"],
        "difficulty": "easy",
        "category": "search",
        "eval_criteria": "Should find references in auth_service.py (register method) and validators.py (validate_password_strength).",
    },
    {
        "id": "test_easy_1",
        "command": "test",
        "args": ["src/utils/validators.py", "--symbol", "validate_email"],
        "difficulty": "easy",
        "category": "test",
        "eval_criteria": "Should generate pytest tests covering: valid emails, missing @, missing domain, special characters, empty string.",
    },
    {
        "id": "test_hard_1",
        "command": "test",
        "args": ["src/services/auth_service.py", "--symbol", "AuthService"],
        "difficulty": "hard",
        "category": "test",
        "eval_criteria": "Should cover registration validation, login success/failure, account locking, rate limiting, session management, password change.",
    },
    {
        "id": "review_staged_1",
        "command": "review",
        "args": ["--staged"],
        "difficulty": "medium",
        "category": "review",
        "setup": "staged_change",
        "eval_criteria": "Should identify security issues: SHA-256 password hashing is weak without salt, missing input validation, potential improvements.",
    },
    {
        "id": "run_plugin_1",
        "command": "run",
        "args": ["security-audit", "audit the authentication and API routes"],
        "difficulty": "medium",
        "category": "run",
        "eval_criteria": "Should identify security concerns: weak password hashing, missing input validation in API routes, session management issues.",
    },

    # ── E2E multi-step coding ──
    {
        "id": "e2e_todo_module",
        "command": "ask",
        "args": ["Create a Python module src/utils/todo.py with a TodoItem dataclass (id: int, title: str, done: bool) and a TodoList class with methods: add(title) -> TodoItem, remove(item_id) -> bool, list_items() -> list, mark_done(item_id) -> bool. Use an in-memory list."],
        "difficulty": "hard",
        "category": "e2e_coding",
        "eval_criteria": "Should generate a complete Python module with: TodoItem dataclass with id/title/done fields, TodoList class with all 4 required methods, proper error handling for missing items, type hints throughout.",
    },
    {
        "id": "e2e_fix_email_validation",
        "command": "ask",
        "args": ["The validate_email regex in src/utils/validators.py is too strict. It rejects valid emails like user@sub.domain.com. Fix the regex to support subdomains properly while still rejecting invalid emails."],
        "difficulty": "medium",
        "category": "e2e_coding",
        "eval_criteria": "Should identify the regex pattern issue and suggest a fix that supports subdomains (e.g., user@sub.domain.com). Should not break existing valid email detection.",
    },
]


# ── Rate-limited HTTP client ──

class EvalClient:
    def __init__(self):
        self.last_request_time = 0

    def _wait(self):
        elapsed = time.time() - self.last_request_time
        if elapsed < REQUEST_INTERVAL:
            time.sleep(REQUEST_INTERVAL - elapsed)
        self.last_request_time = time.time()

    def call(self, system_prompt: str, user_prompt: str) -> str:
        if not EVAL_BASE_URL or not EVAL_API_KEY:
            return '{"error": "EVAL_BASE_URL and EVAL_API_KEY must be set"}'

        self._wait()
        body = json.dumps({
            "model": EVAL_MODEL,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            "stream": False,
            "temperature": 0.1,
        }).encode()

        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {EVAL_API_KEY}",
        }

        for attempt in range(MAX_RETRIES):
            try:
                req = urllib.request.Request(
                    f"{EVAL_BASE_URL}/v1/chat/completions",
                    data=body, headers=headers, method="POST",
                )
                with urllib.request.urlopen(req, timeout=120) as resp:
                    data = json.loads(resp.read().decode())
                    return data["choices"][0]["message"]["content"]
            except urllib.error.HTTPError as e:
                if e.code in (429, 500, 502, 503):
                    wait = RETRY_BACKOFF * (attempt + 1)
                    print(f"    [Retry {attempt+1}/{MAX_RETRIES}: HTTP {e.code}, waiting {wait}s]")
                    time.sleep(wait)
                    continue
                return f'{{"error": "HTTP {e.code}"}}'
            except Exception as e:
                if attempt < MAX_RETRIES - 1:
                    time.sleep(RETRY_BACKOFF)
                    continue
                return f'{{"error": "{e}"}}'
        return '{"error": "Max retries exceeded"}'


# ── Dumbcoder runner ──

def run_dumbcoder(command: str, args: list, timeout: int = 180) -> tuple:
    cmd = [DUMBCODER_BIN, command] + args
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, cwd=WORK_DIR)
        return result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired:
        return "", "TIMEOUT", -1
    except Exception as e:
        return "", str(e), -1


def run_setup(name: str):
    if name == "staged_change":
        path = os.path.join(WORK_DIR, "src", "services", "auth_service.py")
        if os.path.exists(path):
            with open(path) as f:
                content = f.read()
            with open(path, "w") as f:
                f.write("# Modified: Added rate limiting improvements\n" + content)
            subprocess.run(["git", "add", "src/services/auth_service.py"], cwd=WORK_DIR, capture_output=True)
            print("  [Setup: staged modification]")


# ── Evaluation ──

EVAL_SYSTEM = """You are an expert evaluator for an AI coding assistant called "dumbcoder".
Rate the response on a scale of 1-5 for each dimension:
1. **Accuracy** (1-5): Factually correct about the codebase?
2. **Completeness** (1-5): Addresses all evaluation criteria?
3. **Relevance** (1-5): Focused and on-topic?
4. **Usefulness** (1-5): Would a developer find this helpful?

Output your evaluation as JSON:
{
    "accuracy": <1-5>,
    "completeness": <1-5>,
    "relevance": <1-5>,
    "usefulness": <1-5>,
    "overall": <1-5>,
    "strengths": ["..."],
    "weaknesses": ["..."],
    "pass": true/false
}

Set "pass" to true if overall >= 3. Be strict but fair."""


def evaluate(client: EvalClient, tc: dict, response: str) -> dict:
    prompt = f"""## Test Case
ID: {tc['id']}
Command: dumbcoder {tc['command']} {' '.join(tc['args'])}
Difficulty: {tc['difficulty']}

## Evaluation Criteria
{tc['eval_criteria']}

## Agent Response
{response[:3000]}

Please evaluate the response against the criteria."""

    result = client.call(EVAL_SYSTEM, prompt)
    try:
        start = result.find('{')
        end = result.rfind('}') + 1
        if start >= 0 and end > start:
            return json.loads(result[start:end])
    except json.JSONDecodeError:
        pass
    return {"overall": 0, "pass": False, "strengths": [], "weaknesses": ["Failed to parse evaluation"], "raw": result[:500]}


# ── Main ──

def main():
    if not EVAL_BASE_URL or not EVAL_API_KEY:
        print("ERROR: Set EVAL_BASE_URL and EVAL_API_KEY environment variables.")
        print("Example:")
        print("  export EVAL_BASE_URL=https://api.example.com")
        print("  export EVAL_API_KEY=sk-...")
        sys.exit(1)

    cases = TEST_CASES
    if TEST_FILTER:
        cases = [tc for tc in cases if tc["id"].startswith(TEST_FILTER)]
        if not cases:
            print(f"No test cases match filter: {TEST_FILTER}")
            sys.exit(1)

    print("=" * 70)
    print("DUMBCODER AGENT PERFORMANCE EVALUATION")
    print(f"Date: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Binary: {DUMBCODER_BIN}")
    print(f"Work dir: {WORK_DIR}")
    print(f"Eval model: {EVAL_MODEL} @ {EVAL_BASE_URL}")
    print(f"Test cases: {len(cases)}")
    if TEST_FILTER:
        print(f"Filter: {TEST_FILTER}")
    print("=" * 70)

    client = EvalClient()
    results = []

    for i, tc in enumerate(cases):
        print(f"\n[{i+1}/{len(cases)}] {tc['id']} ({tc['difficulty']})")
        print(f"  Command: dumbcoder {tc['command']} {' '.join(tc['args'])}")

        if tc.get("setup"):
            run_setup(tc["setup"])

        t0 = time.time()
        stdout, stderr, rc = run_dumbcoder(tc["command"], tc["args"])
        elapsed = time.time() - t0

        response = stdout.strip()
        if not response and stderr.strip():
            response = f"[stderr]: {stderr.strip()[:500]}"

        print(f"  Time: {elapsed:.1f}s | rc={rc} | {len(response)} chars")

        if rc != 0:
            print(f"  WARNING: non-zero exit (stderr: {stderr[:100]})")

        preview = response[:150].replace('\n', ' ')
        print(f"  Preview: {preview}...")

        print(f"  Evaluating...")
        eval_result = evaluate(client, tc, response)

        overall = eval_result.get("overall", 0)
        passed = eval_result.get("pass", False)
        print(f"  Score: {overall}/5 {'PASS' if passed else 'FAIL'}")

        results.append({
            "test_case": tc,
            "response": response[:2000],
            "elapsed": elapsed,
            "return_code": rc,
            "evaluation": eval_result,
        })

    # ── Summary ──
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)

    total = len(results)
    passed = sum(1 for r in results if r["evaluation"].get("pass", False))
    failed = total - passed

    print(f"\nTotal: {total}  Passed: {passed}  Failed: {failed}  Rate: {passed/total*100:.0f}%")

    for diff in ["easy", "medium", "hard"]:
        subset = [r for r in results if r["test_case"]["difficulty"] == diff]
        if subset:
            p = sum(1 for r in subset if r["evaluation"].get("pass", False))
            print(f"  {diff.capitalize()}: {p}/{len(subset)}")

    print("\nBy Category:")
    for cat in sorted(set(r["test_case"]["category"] for r in results)):
        subset = [r for r in results if r["test_case"]["category"] == cat]
        p = sum(1 for r in subset if r["evaluation"].get("pass", False))
        avg = sum(r["evaluation"].get("overall", 0) for r in subset) / len(subset)
        print(f"  {cat}: {p}/{len(subset)} (avg {avg:.1f}/5)")

    print("\nAverage Scores:")
    for dim in ["accuracy", "completeness", "relevance", "usefulness", "overall"]:
        scores = [r["evaluation"].get(dim, 0) for r in results]
        print(f"  {dim.capitalize()}: {sum(scores)/len(scores):.2f}/5")

    # Failed details
    failed_list = [r for r in results if not r["evaluation"].get("pass", False)]
    if failed_list:
        print(f"\nFailed Tests ({len(failed_list)}):")
        for r in failed_list:
            w = r["evaluation"].get("weaknesses", [])
            print(f"  - {r['test_case']['id']}: {r['evaluation'].get('overall',0)}/5")
            for item in w[:2]:
                print(f"    * {item}")

    # Save report
    report_path = os.path.join(os.path.dirname(WORK_DIR), "test_report.json")
    with open(report_path, "w") as f:
        json.dump({
            "timestamp": datetime.now().isoformat(),
            "config": {
                "binary": DUMBCODER_BIN,
                "work_dir": WORK_DIR,
                "eval_model": EVAL_MODEL,
                "eval_base_url": EVAL_BASE_URL,
            },
            "summary": {"total": total, "passed": passed, "failed": failed, "pass_rate": f"{passed/total*100:.0f}%"},
            "results": results,
        }, f, indent=2, ensure_ascii=False)
    print(f"\nReport: {report_path}")

    return 0 if passed / total >= 0.5 else 1


if __name__ == "__main__":
    sys.exit(main())
