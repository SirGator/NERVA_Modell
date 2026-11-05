from pathlib import Path
from evolution.spec import fields, number, require
from planning.experiment_designer import identifier, integer


class DiagnosisSpec:
    def __init__(self, source, config_folder):
        fields(source, ['diagnosis'])
        s = source.get('diagnosis', {})
        fields(s, ['name', 'archive', 'case', 'training_seeds', 'test_seeds', 'ridge',
                   'time_bins', 'max_records', 'max_features', 'max_new_trials', 'stage'])
        require(identifier(s.get('name')), 'invalid diagnosis name')
        require(s.get('stage', 'retrospective_pilot') == 'retrospective_pilot',
                'archived M9 analysis is a retrospective pilot, not fresh confirmation')
        self.name = s['name']
        require(isinstance(s.get('archive'), str) and s['archive'], 'archive path required')
        self.archive = (Path(config_folder)/s['archive']).resolve()
        require(identifier(s.get('case')), 'one explicit M9 case required')
        self.case = s['case']

        def seeds(key):
            values = s.get(key)
            require(isinstance(values, list) and 2 <= len(values) <= 128, key+' needs 2..128 seeds')
            for value in values:
                integer(value, 0, 2**64-1, key)
            require(len(set(values)) == len(values), 'duplicate '+key)
            return values

        self.training, self.test = seeds('training_seeds'), seeds('test_seeds')
        require(not set(self.training)&set(self.test), 'decoder training/test seeds must be disjoint')
        self.ridge = s.get('ridge', 1.)
        require(number(self.ridge) and self.ridge > 0, 'positive finite ridge regularization required')
        self.bins = integer(s.get('time_bins', 5), 1, 20, 'time_bins')
        self.max_records = integer(s.get('max_records', 1024), 64, 8192, 'max_records')
        self.max_features = integer(s.get('max_features', 256), 1, 1024, 'max_features')
        require(type(s.get('max_new_trials', 0)) is int and s.get('max_new_trials', 0) == 0,
                'archive diagnosis never schedules new neural trials')
        require(32*(len(self.training)+len(self.test)) <= self.max_records, 'diagnostic record budget exceeded')
        self.source = source
