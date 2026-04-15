"""Services module — tests doxr-native cross-reference syntax."""

from pkg.models import User, Admin, helper_func


def native_fq_refs() -> None:
    """FQ bare brackets.

    [pkg.models.User]
    [`pkg.models.Admin`]
    [pkg.models.User.greet]
    [pkg.models.User.name]
    [pkg.models.User.role]
    """
    pass


def native_short_refs() -> None:
    """Short names resolved via imports.

    [User]
    [`User`]
    [Admin]
    [helper_func]
    """
    pass


def native_broken_refs() -> None:
    """Broken refs that should be flagged.

    [Nonexistent]
    [pkg.models.Fake]
    [`AlsoFake`]
    """
    pass


def native_ignored() -> None:
    """These should NOT be treated as refs.

    \[User\]
    [see above]
    [1]
    [some/path]
    """
    pass


def native_mixed() -> None:
    """Mixed native + MkDocs + Sphinx.

    Native: [User]
    MkDocs explicit: [click here][pkg.models.Admin]
    MkDocs autoref: [pkg.models.User][]
    Sphinx: :class:`pkg.models.User`
    Native FQ: [pkg.models.helper_func]
    """
    pass
