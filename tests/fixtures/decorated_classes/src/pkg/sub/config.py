"""Decorated class (e.g., @dataclass)."""
from dataclasses import dataclass


@dataclass
class Config:
    """A dataclass — decorated class definition.

    Valid: [name][pkg.sub.config.Config.name]
    Valid: [value][pkg.sub.config.Config.value]
    Valid (via re-export): [name][pkg.Config.name]
    Invalid: [fake][pkg.sub.config.Config.nonexistent]
    """

    name: str
    value: int = 0
