"""
View layer referencing symbols through wildcard re-exports.

These reference the public API through __init__.py's `from pkg.models import *`:
- [pkg.User][] should resolve (User is in models.__all__)
- [pkg.Admin][] should resolve (Admin is in models.__all__)

These reference helpers (no __all__, so all public names exported):
- [pkg.helper_func][] should resolve
- [pkg.another_helper][] should resolve

Direct paths should always work regardless of wildcards:
- [pkg.models.User][] direct path
- [pkg.models.Admin][] direct path
- [pkg.helpers.helper_func][] direct path
"""
