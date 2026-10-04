"""A typed program over the Pythonic layer (#332). test_typing.py checks
it, with the docs snippets in test_pythonic.py, under ``mypy --strict``
and runs it."""

from typing import Dict, List, Optional, Tuple

import openbim_ifc
from openbim_ifc import Entity, IfcModel, PlainValue, PropertyValue, Text, Typed, Value


def external_walls(model: IfcModel) -> List[Tuple[int, Optional[str]]]:
    """``(id, Name)`` of every wall whose ``Pset_WallCommon.IsExternal`` is
    true, its own value or its type's."""
    found: List[Tuple[int, Optional[str]]] = []
    for wall in model.by_type("IfcWall"):
        common = wall.psets.get("Pset_WallCommon", {})
        if common.get("IsExternal") is True:
            name: PlainValue = wall.get("Name")
            found.append((wall.id, name if isinstance(name, str) else None))
    return found


def rename(entity: Entity, suffix: str) -> Value:
    """Append ``suffix`` to an entity's name; returns the old tagged value."""
    old = entity.raw("Name")
    text = old.value.value if isinstance(old, Typed) and isinstance(old.value, Text) else (
        old.value if isinstance(old, Text) else ""
    )
    entity.Name = Text(text + suffix)
    return old


def property_table(entity: Entity) -> Dict[str, PropertyValue]:
    """Every property and quantity of ``entity`` as ``"Set.Name"``."""
    table: Dict[str, PropertyValue] = {}
    for sets in (entity.psets, entity.qtos):
        for set_name, values in sets.items():
            for name, value in values.items():
                table[f"{set_name}.{name}"] = value
    return table


def main(path: str) -> Tuple[List[Tuple[int, Optional[str]]], int, Dict[str, PropertyValue]]:
    model = openbim_ifc.open(path)
    walls = external_walls(model)
    first = model[walls[0][0]]
    rename(first, " (external)")
    assert first.is_a("IfcBuildingElement") and first in model
    return walls, len(model), property_table(first)
