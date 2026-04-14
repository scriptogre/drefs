"""Abstract base with various method types."""
from abc import ABC, abstractmethod
from typing import Generic, TypeVar

T = TypeVar('T')


class AbstractBase(ABC, Generic[T]):
    """Abstract base class.

    Valid: [process][pkg.sub.base.AbstractBase.process]
    Valid: [class_create][pkg.sub.base.AbstractBase.class_create]
    Valid: [async_method][pkg.sub.base.AbstractBase.async_method]
    Invalid: [nope][pkg.sub.base.AbstractBase.nope]
    """

    @classmethod
    def class_create(cls) -> None:
        """A classmethod."""
        pass

    @abstractmethod
    async def process(self) -> None:
        """Abstract async method."""
        pass

    async def async_method(self) -> None:
        """Regular async method."""
        pass

    def sync_method(self) -> None:
        """Sync method."""
        pass
