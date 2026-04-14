"""Deeply nested module — tests multi-level import chains."""

from pkg.mixins.serializable import SerializableMixin


class DeepClass(SerializableMixin):
    """A deeply nested class.

    ## Valid references:

    Root re-export: [DeepClass][pkg.DeepClass]
    Sub re-export: [DeepClass][pkg.sub.DeepClass]
    Direct path: [DeepClass][pkg.sub.deep.DeepClass]
    Inherited method: [to_json][pkg.sub.deep.DeepClass.to_json]
    Own method: [process][pkg.sub.deep.DeepClass.process]

    ## Invalid references:

    Wrong path: [DeepClass][pkg.deep.DeepClass]
    Nonexistent member: [nope][pkg.sub.deep.DeepClass.nope]
    """

    def process(self) -> None:
        """Process something."""
        pass
