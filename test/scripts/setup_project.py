#!/usr/bin/env python3
"""
Set up the test project for dumbcoder evaluation.
Creates a sample Python e-commerce project with realistic code.
"""

import os
import subprocess
import sys

DEFAULT_WORK_DIR = os.environ.get("DUMBCODER_WORK_DIR", "/opt/workspace/test-project")
DUMBCODER_BIN = os.environ.get(
    "DUMBCODER_BIN",
    os.path.join(os.path.dirname(__file__), "../../target/release/dumbcoder"),
)


def create_project(root: str):
    """Create the test project structure and files."""
    os.makedirs(root, exist_ok=True)

    # Init git repo
    if not os.path.exists(os.path.join(root, ".git")):
        subprocess.run(["git", "init"], cwd=root, capture_output=True)

    # Create directories
    for d in ["src/models", "src/services", "src/api", "src/utils", "tests"]:
        os.makedirs(os.path.join(root, d), exist_ok=True)

    # Create __init__.py files
    for d in ["src", "src/models", "src/services", "src/api", "src/utils", "tests"]:
        init_path = os.path.join(root, d, "__init__.py")
        if not os.path.exists(init_path):
            open(init_path, "w").close()

    # ── src/models/user.py ──
    write_file(root, "src/models/user.py", '''\
import hashlib
import time
from typing import Optional


class User:
    """User model for authentication and profile management."""

    def __init__(self, user_id: int, username: str, email: str, password_hash: str,
                 role: str = "user", created_at: Optional[float] = None):
        self.user_id = user_id
        self.username = username
        self.email = email
        self.password_hash = password_hash
        self.role = role
        self.created_at = created_at or time.time()
        self.is_active = True
        self.last_login: Optional[float] = None
        self.login_attempts = 0
        self.locked_until: Optional[float] = None

    def verify_password(self, password: str) -> bool:
        """Verify password against stored hash."""
        return self.password_hash == hashlib.sha256(password.encode()).hexdigest()

    def is_locked(self) -> bool:
        """Check if account is temporarily locked."""
        if self.locked_until and time.time() < self.locked_until:
            return True
        if self.locked_until and time.time() >= self.locked_until:
            self.locked_until = None
            self.login_attempts = 0
        return False

    def record_failed_login(self) -> None:
        """Record a failed login attempt, lock after 5 failures."""
        self.login_attempts += 1
        if self.login_attempts >= 5:
            self.locked_until = time.time() + 900  # 15 min lock

    def record_successful_login(self) -> None:
        """Record successful login."""
        self.login_attempts = 0
        self.locked_until = None
        self.last_login = time.time()

    def has_permission(self, permission: str) -> bool:
        """Check if user has a specific permission based on role."""
        role_permissions = {
            "admin": ["read", "write", "delete", "manage_users", "view_logs"],
            "editor": ["read", "write"],
            "user": ["read"],
        }
        allowed = role_permissions.get(self.role, [])
        return permission in allowed

    def to_dict(self) -> dict:
        """Serialize user to dictionary (excluding sensitive data)."""
        return {
            "user_id": self.user_id,
            "username": self.username,
            "email": self.email,
            "role": self.role,
            "created_at": self.created_at,
            "is_active": self.is_active,
            "last_login": self.last_login,
        }


class UserRepository:
    """In-memory user storage (simulates database)."""

    def __init__(self):
        self._users: dict[int, User] = {}
        self._next_id = 1
        self._email_index: dict[str, int] = {}
        self._username_index: dict[str, int] = {}

    def create(self, username: str, email: str, password: str, role: str = "user") -> User:
        if email in self._email_index:
            raise ValueError(f"Email {email} already registered")
        if username in self._username_index:
            raise ValueError(f"Username {username} already taken")
        password_hash = hashlib.sha256(password.encode()).hexdigest()
        user = User(self._next_id, username, email, password_hash, role)
        self._users[self._next_id] = user
        self._email_index[email] = self._next_id
        self._username_index[username] = self._next_id
        self._next_id += 1
        return user

    def get_by_id(self, user_id: int) -> Optional[User]:
        return self._users.get(user_id)

    def get_by_email(self, email: str) -> Optional[User]:
        uid = self._email_index.get(email)
        return self._users.get(uid) if uid else None

    def delete(self, user_id: int) -> bool:
        user = self._users.pop(user_id, None)
        if user:
            self._email_index.pop(user.email, None)
            self._username_index.pop(user.username, None)
            return True
        return False

    def list_all(self) -> list:
        return list(self._users.values())
''')

    # ── src/models/order.py ──
    write_file(root, "src/models/order.py", '''\
import time
import uuid
from enum import Enum
from typing import Optional
from dataclasses import dataclass, field


class OrderStatus(Enum):
    PENDING = "pending"
    CONFIRMED = "confirmed"
    PROCESSING = "processing"
    SHIPPED = "shipped"
    DELIVERED = "delivered"
    CANCELLED = "cancelled"
    REFUNDED = "refunded"


@dataclass
class OrderItem:
    product_id: str
    product_name: str
    quantity: int
    unit_price: float

    @property
    def subtotal(self) -> float:
        return self.quantity * self.unit_price


@dataclass
class Order:
    order_id: str = field(default_factory=lambda: str(uuid.uuid4())[:8])
    user_id: int = 0
    items: list[OrderItem] = field(default_factory=list)
    status: OrderStatus = OrderStatus.PENDING
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    shipping_address: str = ""
    notes: str = ""

    @property
    def total_amount(self) -> float:
        return sum(item.subtotal for item in self.items)

    @property
    def item_count(self) -> int:
        return sum(item.quantity for item in self.items)

    def can_transition_to(self, new_status: OrderStatus) -> bool:
        valid_transitions = {
            OrderStatus.PENDING: [OrderStatus.CONFIRMED, OrderStatus.CANCELLED],
            OrderStatus.CONFIRMED: [OrderStatus.PROCESSING, OrderStatus.CANCELLED],
            OrderStatus.PROCESSING: [OrderStatus.SHIPPED, OrderStatus.CANCELLED],
            OrderStatus.SHIPPED: [OrderStatus.DELIVERED],
            OrderStatus.DELIVERED: [OrderStatus.REFUNDED],
            OrderStatus.CANCELLED: [],
            OrderStatus.REFUNDED: [],
        }
        return new_status in valid_transitions.get(self.status, [])

    def transition_to(self, new_status: OrderStatus) -> None:
        if not self.can_transition_to(new_status):
            raise ValueError(f"Cannot transition from {self.status.value} to {new_status.value}")
        self.status = new_status
        self.updated_at = time.time()
''')

    # ── src/services/auth_service.py ──
    write_file(root, "src/services/auth_service.py", '''\
import time
import hashlib
from typing import Optional
from src.models.user import User, UserRepository


class AuthService:
    """Authentication service with rate limiting and session management."""

    def __init__(self, user_repo: UserRepository):
        self.user_repo = user_repo
        self._sessions: dict[str, dict] = {}
        self._rate_limits: dict[str, list[float]] = {}
        self.max_requests_per_minute = 30

    def register(self, username: str, email: str, password: str) -> User:
        if len(password) < 8:
            raise ValueError("Password must be at least 8 characters")
        if not any(c.isupper() for c in password):
            raise ValueError("Password must contain at least one uppercase letter")
        if not any(c.isdigit() for c in password):
            raise ValueError("Password must contain at least one digit")
        if "@" not in email:
            raise ValueError("Invalid email format")
        return self.user_repo.create(username, email, password)

    def login(self, email: str, password: str, ip_address: str = "unknown") -> Optional[str]:
        if self._is_rate_limited(ip_address):
            raise PermissionError("Too many requests. Please try again later.")
        self._record_request(ip_address)
        user = self.user_repo.get_by_email(email)
        if not user:
            return None
        if user.is_locked():
            raise PermissionError("Account is temporarily locked")
        if not user.verify_password(password):
            user.record_failed_login()
            return None
        user.record_successful_login()
        token = f"session_{user.user_id}_{int(time.time())}"
        self._sessions[token] = {"user_id": user.user_id, "created_at": time.time(), "ip_address": ip_address}
        return token

    def validate_session(self, token: str) -> Optional[User]:
        session = self._sessions.get(token)
        if not session:
            return None
        if time.time() - session["created_at"] > 86400:
            del self._sessions[token]
            return None
        return self.user_repo.get_by_id(session["user_id"])

    def logout(self, token: str) -> bool:
        return self._sessions.pop(token, None) is not None

    def _is_rate_limited(self, ip_address: str) -> bool:
        now = time.time()
        timestamps = self._rate_limits.get(ip_address, [])
        recent = [t for t in timestamps if now - t < 60]
        self._rate_limits[ip_address] = recent
        return len(recent) >= self.max_requests_per_minute

    def _record_request(self, ip_address: str) -> None:
        if ip_address not in self._rate_limits:
            self._rate_limits[ip_address] = []
        self._rate_limits[ip_address].append(time.time())

    def change_password(self, token: str, old_password: str, new_password: str) -> bool:
        user = self.validate_session(token)
        if not user:
            raise PermissionError("Invalid session")
        if not user.verify_password(old_password):
            return False
        if len(new_password) < 8:
            raise ValueError("Password must be at least 8 characters")
        user.password_hash = hashlib.sha256(new_password.encode()).hexdigest()
        return True
''')

    # ── src/services/order_service.py ──
    write_file(root, "src/services/order_service.py", '''\
from src.models.order import Order, OrderItem, OrderStatus, OrderRepository
from src.models.user import User


class OrderService:
    def __init__(self, order_repo: OrderRepository):
        self.order_repo = order_repo

    def create_order(self, user: User, items_data: list[dict], shipping_address: str) -> Order:
        if not items_data:
            raise ValueError("Order must have at least one item")
        items = []
        for d in items_data:
            if d.get("quantity", 0) <= 0:
                raise ValueError(f"Invalid quantity for {d.get('product_id')}")
            items.append(OrderItem(product_id=d["product_id"], product_name=d.get("product_name", ""),
                                   quantity=d["quantity"], unit_price=d["unit_price"]))
        order = self.order_repo.create(user.user_id, items, shipping_address)
        return order

    def cancel_order(self, user: User, order_id: str, reason: str = "") -> Order:
        order = self.order_repo.get_by_id(order_id)
        if not order:
            raise ValueError(f"Order {order_id} not found")
        if order.user_id != user.user_id and not user.has_permission("manage_users"):
            raise PermissionError("No permission to cancel this order")
        order.transition_to(OrderStatus.CANCELLED)
        if reason:
            order.notes = f"Cancelled: {reason}"
        return order

    def refund_order(self, user: User, order_id: str, reason: str = "") -> Order:
        order = self.order_repo.get_by_id(order_id)
        if not order:
            raise ValueError(f"Order {order_id} not found")
        if order.user_id != user.user_id and not user.has_permission("manage_users"):
            raise PermissionError("No permission to refund this order")
        order.transition_to(OrderStatus.REFUNDED)
        order.notes = f"Refunded: {reason}" if reason else "Refunded"
        return order

    def get_order_summary(self, user: User) -> dict:
        orders = self.order_repo.get_user_orders(user.user_id)
        return {
            "total_orders": len(orders),
            "total_spent": sum(o.total_amount for o in orders if o.status == OrderStatus.DELIVERED),
            "pending": sum(1 for o in orders if o.status in [OrderStatus.PENDING, OrderStatus.CONFIRMED]),
        }
''')

    # ── src/utils/validators.py ──
    write_file(root, "src/utils/validators.py", '''\
import re


def validate_email(email: str) -> bool:
    pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$'
    return bool(re.match(pattern, email))


def validate_password_strength(password: str) -> dict:
    issues = []
    score = 0
    if len(password) >= 8: score += 1
    else: issues.append("Password must be at least 8 characters")
    if len(password) >= 12: score += 1
    if re.search(r'[A-Z]', password): score += 1
    else: issues.append("Missing uppercase letter")
    if re.search(r'[a-z]', password): score += 1
    else: issues.append("Missing lowercase letter")
    if re.search(r'\\d', password): score += 1
    else: issues.append("Missing digit")
    if re.search(r'[!@#$%^&*(),.?":{}|<>]', password): score += 1
    else: issues.append("Missing special character")
    common = ["password", "123456", "qwerty", "admin", "letmein"]
    if password.lower() in common:
        issues.append("Commonly used password")
        score = 0
    strength = "weak" if score < 3 else ("medium" if score < 5 else "strong")
    return {"valid": len(issues) == 0, "strength": strength, "score": score, "issues": issues}


def sanitize_input(text: str, max_length: int = 1000) -> str:
    if not text: return ""
    text = text.replace('\\x00', '')
    return text[:max_length].strip()
''')

    # Commit
    subprocess.run(["git", "add", "-A"], cwd=root, capture_output=True)
    subprocess.run(["git", "commit", "-m", "Initial project setup"], cwd=root, capture_output=True)
    print(f"Test project created at {root}")


def setup_dumbcoder(root: str):
    """Initialize dumbcoder and configure it."""
    # Run dumbcoder init
    subprocess.run([DUMBCODER_BIN, "init"], cwd=root, capture_output=True)

    # Write config
    eval_provider = os.environ.get("DUMBCODER_PROVIDER", "openai_compatible")
    eval_base_url = os.environ.get("DUMBCODER_BASE_URL", "https://api.siliconflow.cn")
    eval_model = os.environ.get("DUMBCODER_MODEL", "Qwen/Qwen3-8B")
    eval_api_key = os.environ.get("DUMBCODER_API_KEY", "")
    context_limit = os.environ.get("DUMBCODER_CONTEXT_LIMIT", "4000")

    config_path = os.path.join(root, ".dumbcoder", "config.toml")
    with open(config_path, "w") as f:
        f.write(f'''[model]
provider = "{eval_provider}"
base_url = "{eval_base_url}"
model = "{eval_model}"
api_key = "{eval_api_key}"
timeout_seconds = 120
context_limit = {context_limit}

[index]
enabled = true
db_path = ".dumbcoder/index"
ignore = [".git", "target", "node_modules", "dist", "build", ".dumbcoder"]

[security]
allow_write = false
allow_network = false

[commands]
allow = ["rg", "git status", "git diff", "git log", "git show"]

[prompts]
''')

    # Create plugin
    plugins_dir = os.path.join(root, ".dumbcoder", "plugins")
    os.makedirs(plugins_dir, exist_ok=True)
    with open(os.path.join(plugins_dir, "security-audit.toml"), "w") as f:
        f.write('''name = "security-audit"
description = "Audit code for security vulnerabilities"
system_prompt = """
You are a security auditor. Analyze the code context for security vulnerabilities.
Focus on: SQL injection, XSS, path traversal, authentication bypass, secrets in code.
Output a structured report with severity levels (Critical/High/Medium/Low).
"""
''')

    # Build index
    subprocess.run([DUMBCODER_BIN, "index", "--full"], cwd=root, capture_output=True)
    print(f"Dumbcoder configured at {root}")


def write_file(root: str, rel_path: str, content: str):
    path = os.path.join(root, rel_path)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(content)


if __name__ == "__main__":
    work_dir = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_WORK_DIR
    create_project(work_dir)
    setup_dumbcoder(work_dir)
