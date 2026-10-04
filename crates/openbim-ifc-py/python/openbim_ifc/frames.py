"""The optional pandas export (#332): :meth:`IfcModel.to_dataframe`.

pandas is the ``pandas`` extra (``pip install 'openbim-ifc[pandas]'``) and
is imported only when a frame is built, so the package works without it.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Dict, List, Tuple

from .entity import _sets

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
    for entity in model.by_type(type_name, include_subtypes=include_subtypes):
        row: Dict[str, Any] = {TYPE_COLUMN: entity.type}
        for name in attributes:
            row[name] = entity.get(name)
        if psets or qtos:
            own_psets, own_qtos = _sets(model, entity.id)
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
