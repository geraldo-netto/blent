"""Current shared setup functions, sourced unchanged; callers supply private paths."""
from pathlib import Path
import shlex

ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'source ' + shlex.quote(str(ROOT / 'scripts/setup-system.sh')) + '\n'
