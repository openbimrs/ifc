"""Records beyond attribute values: read options, the STEP header,
validation reports and unreachable products. All frozen dataclasses.

Each mirrors one record of the shared binding core, so the JavaScript and
C bindings carry the same fields under their own names.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Optional, Tuple


@dataclass(frozen=True)
class ParseOptions:
    """How a STEP read treats damaged input. The defaults are strict.

    ``on_malformed="skip"`` drops a record that cannot be parsed and reports
    it in :meth:`IfcModel.diagnostics` instead of failing the read.
    ``check_references`` reports duplicate ids and references to undefined
    ids. ``accept_real_without_point`` reads ``1E-05`` as a real.
    """

    on_malformed: str = "abort"
    check_references: bool = False
    accept_real_without_point: bool = False

    @classmethod
    def strict(cls) -> "ParseOptions":
        """Any malformed record fails the read."""
        return cls()

    @classmethod
    def lenient(cls) -> "ParseOptions":
        """Skip malformed records and accept reals without a point."""
        return cls(on_malformed="skip", accept_real_without_point=True)

    def _keywords(self) -> Dict[str, Any]:
        return {
            "on_malformed": self.on_malformed,
            "check_references": self.check_references,
            "accept_real_without_point": self.accept_real_without_point,
        }


@dataclass(frozen=True)
class Header:
    """The STEP file header: ``FILE_DESCRIPTION``, ``FILE_NAME``, ``FILE_SCHEMA``.

    Change a field with ``dataclasses.replace`` and pass the result to
    :meth:`IfcModel.set_header`.
    """

    description: Tuple[str, ...]
    implementation_level: str
    name: str
    time_stamp: str
    author: Tuple[str, ...]
    organization: Tuple[str, ...]
    preprocessor_version: str
    originating_system: str
    authorization: str
    schema: Tuple[str, ...]

    def __post_init__(self) -> None:
        for name in ("description", "author", "organization", "schema"):
            object.__setattr__(self, name, tuple(getattr(self, name)))

    @classmethod
    def _from_wire(cls, data: Dict[str, Any]) -> "Header":
        return cls(**data)

    def _to_wire(self) -> Dict[str, Any]:
        return {
            "description": list(self.description),
            "implementation_level": self.implementation_level,
            "name": self.name,
            "time_stamp": self.time_stamp,
            "author": list(self.author),
            "organization": list(self.organization),
            "preprocessor_version": self.preprocessor_version,
            "originating_system": self.originating_system,
            "authorization": self.authorization,
            "schema": list(self.schema),
        }


@dataclass(frozen=True)
class ValidationFinding:
    """One finding. ``entity`` is ``None`` for the file as a whole.

    ``severity`` is ``error``, ``evaluation-error``, ``warning`` or
    ``unsupported``; ``rule`` is the check's stable id.
    """

    severity: str
    rule: str
    entity: Optional[int]
    attribute_index: Optional[int]
    attribute_name: Optional[str]
    path: str
    message: str


@dataclass(frozen=True)
class ValidationReport:
    """The result of :meth:`IfcModel.validate`.

    ``conformant`` means no errors and no evaluation errors; unsupported
    rules do not count against it. ``truncated`` means the finding budget was
    reached, so the counts are lower bounds.
    """

    conformant: bool
    truncated: bool
    errors: int
    evaluation_errors: int
    warnings: int
    unsupported: int
    findings: Tuple[ValidationFinding, ...]

    @classmethod
    def _from_wire(cls, data: Dict[str, Any]) -> "ValidationReport":
        fields = dict(data)
        fields["findings"] = tuple(ValidationFinding(**row) for row in data["findings"])
        return cls(**fields)


@dataclass(frozen=True)
class UnreachableProduct:
    """A product no viewer will draw.

    ``reason`` is ``not-contained-in-spatial-structure``,
    ``no-representation-in-model-context`` or
    ``representation-without-context``; ``found_views`` lists the target
    views the geometry sits in instead, for the second reason.
    """

    id: int
    reason: str
    found_views: Tuple[str, ...]
    message: str

    def __post_init__(self) -> None:
        object.__setattr__(self, "found_views", tuple(self.found_views))
