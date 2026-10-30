// Entry point so that `node --test tests/e2e/panel/` works: Node 22 resolves a
// directory argument to this file. It loads every unit test file here (none
// needs a browser). Equivalent: node --test "tests/e2e/panel/*.test.mjs"
import './score.test.mjs';
import './report.test.mjs';
import './harden.test.mjs';
