"""Mixin providing serialization methods."""


class SerializableMixin:
    """A mixin that adds JSON serialization."""

    def to_json(self) -> str:
        """Serialize to JSON string."""
        return "{}"

    def from_json(self, data: str) -> None:
        """Deserialize from JSON string."""
        pass
