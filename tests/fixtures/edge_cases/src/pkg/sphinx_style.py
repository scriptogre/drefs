"""Module using Sphinx-style cross-references."""

from pkg.models import User


def sphinx_func() -> None:
    """A function with Sphinx-style refs.

    ## Valid references:

    :class:`pkg.models.User`
    :func:`pkg.sub.helpers.helper_func`
    :meth:`pkg.models.User.greet`
    :mod:`pkg.sub`
    :class:`~pkg.models.Admin`
    :attr:`pkg.models.User.role`

    ## Invalid references:

    :class:`pkg.models.FakeClass`
    :func:`pkg.nonexistent.bad_func`
    :meth:`pkg.models.User.nonexistent_method`
    """
    pass
