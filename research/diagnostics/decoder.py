"""Fixed linear ridge probe. Scaling and coefficients use training rows only."""
import math
import statistics
from evolution.spec import number, require


def dot(a, b):
    return sum(x*y for x, y in zip(a, b))


def solve(matrix, target):
    """Pivoted Gaussian elimination for the small positive-definite dual system."""
    n = len(target)
    rows = [list(row)+[y] for row, y in zip(matrix, target)]
    for column in range(n):
        pivot = max(range(column, n), key=lambda r: abs(rows[r][column]))
        rows[column], rows[pivot] = rows[pivot], rows[column]
        require(abs(rows[column][column]) > 1e-12, 'singular diagnostic decoder system')
        scale = rows[column][column]
        rows[column] = [v/scale for v in rows[column]]
        for row in range(column+1, n):
            factor = rows[row][column]
            for j in range(column, n+1):
                rows[row][j] -= factor*rows[column][j]
    answer = [0.]*n
    for i in range(n-1, -1, -1):
        answer[i] = rows[i][-1]-sum(rows[i][j]*answer[j] for j in range(i+1, n))
    require(all(math.isfinite(x) for x in answer), 'nonfinite decoder coefficients')
    return answer


def fit(features, labels, ridge):
    require(features and len(features) == len(labels), 'decoder needs paired training rows')
    width = len(features[0])
    require(width and all(len(r) == width and all(number(v) for v in r) for r in features),
            'invalid decoder training features')
    require(set(labels) == {-1, 1}, 'decoder needs both target classes')
    means = [statistics.mean(r[j] for r in features) for j in range(width)]
    scales = [statistics.pstdev(r[j] for r in features) or 1. for j in range(width)]
    normalized = [[(v-means[j])/scales[j] for j, v in enumerate(row)] for row in features]
    intercept = statistics.mean(labels)
    n = len(features)
    gram = [[dot(a, b)+(n*ridge if i == j else 0.) for j, b in enumerate(normalized)]
            for i, a in enumerate(normalized)]
    dual = solve(gram, [y-intercept for y in labels])
    coefficients = [sum(alpha*row[j] for alpha, row in zip(dual, normalized)) for j in range(width)]
    return dict(means=means, scales=scales, coefficients=coefficients, intercept=intercept,
                ridge=ridge, objective='mean squared prediction error + ridge * squared coefficient norm',
                training_rows=n)


def predict(model, features):
    require(len(features) == len(model['means']) and all(number(v) for v in features), 'invalid test features')
    row = [(v-m)/s for v, m, s in zip(features, model['means'], model['scales'])]
    return 1 if dot(model['coefficients'], row)+model['intercept'] >= 0 else -1
