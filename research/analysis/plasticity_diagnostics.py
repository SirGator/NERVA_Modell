"""Identical 2x2 development/homeostasis diagnosis at every requested size."""
import statistics
from contracts import ExperimentPlan
from evolution.spec import fields, require
from .size_environment import SizeEnvironment, sizes


class PlasticityDiagnostics(SizeEnvironment):
    def __init__(self, options):
        fields(options,["sizes"])
        self.sizes=sizes(options)

    def propose(self, context):
        return ExperimentPlan("plasticity_diagnostics",[
            dict(context.base,processing_neurons=n,**{
                "mechanisms.development":development,"mechanisms.homeostasis":homeostasis})
            for n in self.sizes for development in [False,True] for homeostasis in [False,True]],
            dict(sizes=self.sizes,metric="visual_accuracy",stdp="unchanged and enabled; matched all-frozen controls"))

    def analyze(self, results):
        report=super().analyze(results)
        report.method="plasticity_diagnostics"
        contrasts=[]
        for n in self.sizes:
            cells=[c for c in report.findings["cells"] if c["processing_neurons"]==n]
            by_flags={(c["parameters"]["mechanisms.development"],c["parameters"]["mechanisms.homeostasis"]):c for c in cells}
            require(set(by_flags)=={(False,False),(False,True),(True,False),(True,True)},"incomplete diagnostic factorial")
            seeds=cells[0]["seeds"]
            require(all(c["seeds"]==seeds for c in cells),"diagnostic contrasts need paired seeds")
            require(all(c["checks"]["matched_experience"] for c in cells),"diagnostic experience mismatch")
            fingerprints=[r["evaluation"]["pairing_fingerprints"] for r in results if r["parameters"]["processing_neurons"]==n]
            require(all(f==fingerprints[0] for f in fingerprints),"diagnostic initial networks or sensory experience differ")
            endpoints={}
            for metric in ["visual_accuracy","prediction_accuracy","position_mae","final_synapses","observed_events"]:
                values={flags:[r["groups"]["active"][metric] for r in c["per_seed"]] for flags,c in by_flags.items()}
                diffs={
                    "development_without_homeostasis":[a-b for a,b in zip(values[True,False],values[False,False])],
                    "development_with_homeostasis":[a-b for a,b in zip(values[True,True],values[False,True])],
                    "homeostasis_without_development":[a-b for a,b in zip(values[False,True],values[False,False])],
                    "homeostasis_with_development":[a-b for a,b in zip(values[True,True],values[True,False])],
                }
                diffs["interaction"]=[a-b for a,b in zip(diffs["development_with_homeostasis"],diffs["development_without_homeostasis"])]
                endpoints[metric]={label:dict(mean=statistics.mean(v),per_seed=v) for label,v in diffs.items()}
            contrasts.append(dict(processing_neurons=n,seeds=seeds,endpoints=endpoints))
        report.findings["contrasts"]=contrasts
        report.limitations += [
            "Development switches growth and pruning together; this does not isolate pruning alone.",
            "Contrasts describe freshly trained matched networks, not acute ablations of a shared trained state.",
            "Post-training visual accuracy is compared with matched aged no-learning controls, not a pre-training checkpoint.",
            "Same protocol and normalized rate definitions apply at every size; passing or failing a functional gate is a measured outcome."]
        return report
