"""Models module — tests multiple resolution patterns."""

from pkg.mixins.serializable import SerializableMixin


class User(SerializableMixin):
    """A user model.

    ## Valid references (should NOT be flagged):

    Direct class ref: [Admin][pkg.models.Admin]
    Re-exported ref: [User][pkg.User]
    Method on this class: [greet][pkg.models.User.greet]
    Inherited method: [to_json][pkg.models.User.to_json]
    Init attribute: [name][pkg.models.User.name]
    Submodule class: [DeepClass][pkg.sub.deep.DeepClass]
    Relative import target: [helper][pkg.sub.helpers.helper_func]

    ## Invalid references (SHOULD be flagged):

    Typo in module: [Ghost][pkg.mdoels.User]
    Nonexistent class: [Nope][pkg.models.NonExistent]
    Nonexistent method: [bad][pkg.models.User.nonexistent_method]
    Nonexistent module: [gone][pkg.totally_fake.Thing]
    """

    role: str = "user"

    def __init__(self, name: str, email: str) -> None:
        self.name = name
        self.email = email
        self.is_active = True

    def greet(self) -> str:
        """Return a greeting.

        See also: [User][pkg.models.User]
        """
        return f"Hello, {self.name}!"


class Admin(User):
    """Admin user.

    Inherits from [User][pkg.models.User].
    Can also be referenced as [Admin][pkg.Admin] via re-export.
    Has inherited method: [to_json][pkg.models.Admin.to_json]
    Has inherited init attr: [name][pkg.models.Admin.name]
    """

    level: int = 1
