"""The optional pandas export (#332): :meth:`IfcModel.to_dataframe`.

pandas is the ``pandas`` extra (``pip install 'openbim-ifc[pandas]'``) and
is imported only when a frame is built, so the package works without it.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Dict, List, Tuple

from ._native import IfcError
from .entity import _split

if TYPE_CHECKING:
    import pandas

    from .model import IfcModel

TYPE_COLUMN = "type"


def to_dataframe(
    model: "IfcModel",
    type_name: str,
    include_subtypes: bool,
    attributes: Tuple[str, ...],
    psets: bool,
    qtos: bool,
) -> "pandas.DataFrame":
    """Entities × attributes, properties and quantities; see
    :meth:`IfcModel.to_dataframe`."""
    try:
        import pandas
    except ImportError as error:
        raise ImportError(
            "IfcModel.to_dataframe needs pandas; install it with "
            "`pip install 'openbim-ifc[pandas]'`"
        ) from error

    columns: Dict[str, None] = dict.fromkeys((TYPE_COLUMN, *attributes))
    if len(columns) != 1 + len(attributes):
        raise ValueError(f"attributes {attributes!r} repeat a name or name the {TYPE_COLUMN!r} column")
    ids: List[int] = []
    rows: List[Dict[str, Any]] = []
    entities = model.by_type(type_name, include_subtypes=include_subtypes)
    # Every row's sets in one pass (#358): linear in the model, not
    # quadratic as one `property_sets` call per row would be.
    many = (
        model.property_sets_many([entity.id for entity in entities])
        if psets or qtos
        else []
    )
    for index, entity in enumerate(entities):
        row: Dict[str, Any] = {TYPE_COLUMN: entity.type}
        for name in attributes:
            row[name] = entity.get(name)
        if psets or qtos:
            answer = many[index]
            if answer.refusal is not None:
                error = IfcError(answer.refusal.message)
                setattr(error, "code", answer.refusal.code)
                raise error
            own_psets, own_qtos = _split(model, answer.sets)
            for wanted, sets in ((psets, own_psets), (qtos, own_qtos)):
                if not wanted:
                    continue
                for set_name, values in sets.items():
                    for prop, value in values.items():
                        column = f"{set_name}.{prop}"
                        if column in row:
                            raise ValueError(
                                f"#{entity.id}: column {column!r} would hold two values"
                            )
                        row[column] = value
                        columns.setdefault(column)
        ids.append(entity.id)
        rows.append(row)
    return pandas.DataFrame(
        rows, index=pandas.Index(ids, name="id", dtype="int64"), columns=list(columns)
    )
