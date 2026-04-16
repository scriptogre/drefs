"""Models module."""

__all__ = ["User", "Admin"]


class User:
    """The user model."""

    def greet(self):
        """Say hello."""
        pass


class Admin(User):
    """Admin user."""

    pass


class _InternalModel:
    """Should NOT be accessible via pkg._InternalModel when __all__ is set."""

    pass
