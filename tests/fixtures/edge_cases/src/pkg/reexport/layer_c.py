"""Origin of the chained re-export."""


class ChainedClass:
    """A class that gets re-exported through two layers.

    ## Valid references:

    Direct: [ChainedClass][pkg.reexport.layer_c.ChainedClass]
    Via layer_b: [ChainedClass][pkg.reexport.layer_b.ChainedClass]
    Via reexport init: [ChainedClass][pkg.reexport.ChainedClass]
    """

    pass
