"""Shared, simulation-independent contracts for research methods."""
from dataclasses import dataclass, field
from typing import Protocol


@dataclass(frozen=True)
class ExperimentContext:
    name: str
    base: dict
    seeds: tuple
    design_seed: int = 42


@dataclass
class ExperimentPlan:
    method: str
    points: list
    design: dict = field(default_factory=dict)


@dataclass
class AnalysisReport:
    method: str
    findings: dict
    limitations: list
    status: str = "exploratory"


class ResearchMethod(Protocol):
    def propose(self, context: ExperimentContext) -> ExperimentPlan: ...
    def analyze(self, results: list) -> AnalysisReport: ...
