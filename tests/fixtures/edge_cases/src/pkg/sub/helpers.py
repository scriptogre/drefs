"""Helper functions — tests function-level refs and relative imports."""

from ..models import User


def helper_func(user: User) -> str:
    """Format a user for display.

    ## Valid references:

    Relative import target: [User][pkg.models.User]
    Function in same module: [another_helper][pkg.sub.helpers.another_helper]

    ## Invalid references:

    Nonexistent function: [bad][pkg.sub.helpers.nonexistent_func]
    """
    return f"{user.name} <{user.email}>"


def another_helper() -> None:
    """Another helper function."""
    pass
