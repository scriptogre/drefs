"""Root package that re-exports from submodules."""

from pkg.models import User, Admin
from pkg.sub.deep import DeepClass
from pkg.mixins.serializable import SerializableMixin

__all__ = ["User", "Admin", "DeepClass", "SerializableMixin"]
