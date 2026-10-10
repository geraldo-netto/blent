"""T827: permanent bad-edge fixtures and checks over maintained source."""
from pathlib import Path
import unittest
from contract_rules import C_OWNERS, FORBIDDEN, violations

ROOT = Path(__file__).resolve().parents[2]


class ArchitectureRulesTest(unittest.TestCase):
    def test_t827_current_edges_respect_contract_direction(self):
        paths = set(FORBIDDEN)
        paths.update(str(p.relative_to(ROOT)) for p in (ROOT / 'host/evdi').glob('*.[ch]'))
        for name in sorted(paths):
            with self.subTest(name=name):
                self.assertEqual(violations(name, (ROOT / name).read_text()), [])

    def test_t827_forbidden_edges_fail_even_with_aliases_or_groups(self):
        for name, symbols in FORBIDDEN.items():
            for symbol in symbols:
                for source in (f'use crate::{symbol} as Hidden;', f'use crate::{{{symbol}}};', f'type Hidden = crate::{symbol};'):
                    with self.subTest(name=name, source=source):
                        self.assertTrue(violations(name, source))
        self.assertTrue(violations('host/src/selection_types.rs', 'include!("worker.rs");'))

    def test_t827_contract_cycle_is_rejected_at_return_edge(self):
        # media may consume selection values; selection may never consume media.
        self.assertEqual(violations('host/src/media.rs', 'use crate::selection::Key;'), [])
        self.assertTrue(violations('host/src/selection_types.rs', 'use crate::media::EncoderSettings;'))

    def test_t827_private_layout_has_explicit_owner_allowlist(self):
        for header, owners in C_OWNERS.items():
            for owner in owners:
                self.assertEqual(violations('host/evdi/' + owner, f'#include "{header}"'), [])
            self.assertTrue(violations('host/evdi/new_adapter.c', f'#include "{header}"'))
        for declaration in ['pthread_mutex_t mutex;', 'struct frame_exchange { int value; };']:
            self.assertTrue(violations('host/evdi/frame_exchange.h', declaration))
        self.assertTrue(violations('host/evdi/capture.c', 'owner -> frames -> mutex;'))

    def test_t827_comments_do_not_add_dependencies(self):
        for name, symbols in FORBIDDEN.items():
            words = ' '.join(symbols)
            self.assertEqual(violations(name, f'// {words}\n/* outer /* {words} */ end */'), [])
        self.assertEqual(violations('host/evdi/frame_exchange.h', '/* pthread_mutex_t hidden; */'), [])


if __name__ == '__main__':
    unittest.main()
