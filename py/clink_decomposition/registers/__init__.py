"""Discipline registers for the decomposition system"""

from .base import (
    DisciplineRegister, 
    register_discipline,
    list_disciplines,
    get_discipline,
    _DISCIPLINE_REGISTRY
)
from .set_theory import SetTheoryRegister
from .category_theory import CategoryTheoryRegister

# Register disciplines
register_discipline("set_theory", SetTheoryRegister)
register_discipline("category_theory", CategoryTheoryRegister)


__all__ = [
    "DisciplineRegister",
    "register_discipline",
    "SetTheoryRegister",
    "CategoryTheoryRegister",
    "list_disciplines",
    "get_discipline",
    "_DISCIPLINE_REGISTRY",
]
