"""Models module with classes and functions."""


class User:
    """A user."""

    role: str = "user"

    def __init__(self, name: str) -> None:
        self.name = name

    def greet(self) -> str:
        """Return a greeting."""
        return f"Hello, {self.name}!"


class Admin(User):
    """An admin user."""

    level: int = 1


def helper_func() -> None:
    """A helper function."""
    pass
