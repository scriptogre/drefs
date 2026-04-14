"""Models for the application."""


class User:
    """A user in the system.

    See [format_name][my_pkg.utils.helpers.format_name] for name formatting.
    See [Admin][my_pkg.models.Admin] for the admin subclass.
    See [Ghost][my_pkg.nonexistent.Ghost] for something that doesn't exist.
    """

    name: str
    email: str

    def greet(self) -> str:
        """Return a greeting.

        Uses :func:`my_pkg.utils.helpers.format_name` internally.
        Also references :class:`my_pkg.fake.Module` which is broken.
        """
        return f"Hello, {self.name}!"


class Admin(User):
    """An admin user.

    Inherits from [User][my_pkg.models.User].
    """

    role: str = "admin"
