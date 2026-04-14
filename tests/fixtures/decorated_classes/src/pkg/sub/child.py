"""Concrete child class inheriting from AbstractBase via subscript."""
from .base import AbstractBase


class ConcreteChild(AbstractBase[int]):
    """Inherits from AbstractBase[int].

    Valid (own method): [do_stuff][pkg.sub.child.ConcreteChild.do_stuff]
    Valid (inherited): [process][pkg.sub.child.ConcreteChild.process]
    Valid (inherited classmethod): [class_create][pkg.sub.child.ConcreteChild.class_create]
    Valid (inherited async): [async_method][pkg.sub.child.ConcreteChild.async_method]
    Valid (via re-export): [do_stuff][pkg.sub.ConcreteChild.do_stuff]
    Valid (via re-export inherited): [process][pkg.sub.ConcreteChild.process]
    Valid (via root re-export): [do_stuff][pkg.ConcreteChild.do_stuff]
    Invalid: [fake][pkg.sub.child.ConcreteChild.fake_method]
    """

    async def process(self) -> None:
        """Implements abstract method."""
        pass

    def do_stuff(self) -> None:
        """Own method."""
        pass
