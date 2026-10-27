#!/usr/bin/env python3
from ci_policy import check

print('CI_POLICY_COMPLETE runner_cells=' + str(check()) + ' maximum_main_lanes=5')