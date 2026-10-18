#!/usr/bin/env bash
# Tests of the unattended-run scripts that need no cloud: chain.sh with the wall and the ledger (run.sh), wall-node.sh
# and wall.sh, against a stand-in for the aws CLI (test/bin/aws: S3 as a directory, the group as a file) and for sudo
# and shutdown.  Needs the release build of the solver (cd rust && cargo build --release; or TAXMAN_BIN names another
# build of the same sources, say the one `cargo build --profile test` makes) and rsync.  sweep.sh's scenario also runs
# the real verify.sh on the sweep's files, which replays the 3,440 committed games three times.
#
#   rust/tools/cloud/test/run.sh [SCENARIO...]     all of them, or the named (chain, ledger, node, launch, sweep)
#
# Prints `ok` or `FAIL` for each check and exits with the number of failures.  A few minutes: the chains are small.

set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
real_cloud="$(cd "$here/.." && pwd)"
crate="$(cd "$real_cloud/../.." && pwd)"
export TAXMAN_BIN="${TAXMAN_BIN:-$crate/target/release/taxman}"
[ -x "$TAXMAN_BIN" ] || {
    echo "build the solver first: cd $crate && cargo build --release" >&2
    exit 2
}
command -v rsync >/dev/null 2>&1 || {
    echo "rsync is needed (the stand-in for aws s3 sync)" >&2
    exit 2
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"; kill $(jobs -p) 2>/dev/null' EXIT
# The scripts under test are a copy of the directory's, but for verify.sh, which replays the 3,440 committed games
# (half a minute) every time a chain completes.
cloud="$tmp/cloud"
mkdir -p "$cloud"
cp "$real_cloud"/*.sh "$real_cloud/wall-policy.json" "$cloud/"
printf '#!/usr/bin/env bash\necho "verify.sh (a stand-in): $*"\n' >"$cloud/verify.sh"
cp "$cloud/verify.sh" "$tmp/verify.sh.stub"
# ... and setup.sh, which installs and builds: the stand-in records how it was called and may sleep a while.
printf '%s\n' '#!/usr/bin/env bash' \
    'echo "setup.sh: NO_PULL=${NO_PULL:-} SKIP_TESTS=${SKIP_TESTS:-} BUILD=${BUILD-unset}" \
        "RELEASE_TESTS=${RELEASE_TESTS:-}" >>"$TAXMAN_WORK/setup-stub.log"' \
    '[ -z "${STUB_SETUP_SLEEP:-}" ] || sleep "$STUB_SETUP_SLEEP"' \
    '[ -z "${STUB_SETUP_FAIL:-}" ] || exit 1' >"$cloud/setup.sh"
chmod +x "$cloud"/*.sh
export TAXMAN_ROOT="$(cd "$crate/.." && pwd)"
export PATH="$here/bin:$PATH"
export FAKE_S3="$tmp/s3" FAKE_STATE="$tmp/state" FAKE_LOG="$tmp/aws.log"
export AWS_DEFAULT_REGION=us-east-1
mkdir -p "$FAKE_S3" "$FAKE_STATE"
failures=0

check() {
    local what="$1"
    shift
    if "$@"; then
        printf 'ok   %s\n' "$what"
    else
        printf 'FAIL %s\n' "$what"
        failures=$((failures + 1))
    fi
}

# The object NAME of the fake bucket b.
obj() {
    printf '%s/b/%s\n' "$FAKE_S3" "$1"
}

# Whether the file $1 holds the line (fixed text) $2 / the pattern $2.
has_line() { grep -qxF -- "$2" "$1" 2>/dev/null; }
matches() { grep -q -- "$2" "$1" 2>/dev/null; }
lacks() { [ -e "$1" ] && ! grep -q -- "$2" "$1"; }
exists() { [ -e "$1" ]; }
absent() { [ ! -e "$1" ]; }
equals() { [ "$1" = "$2" ]; }
# Whether the directory $1 has a file whose name matches $2.
lists() { ls "$1" 2>/dev/null | grep -q -- "$2"; }

# The seconds the fake machine has been up and its boot, which run.sh reads from these.
machine() {
    export UPTIME_SECONDS="$1" BOOT_ID="$2"
}

wait_for() {
    local tries=0
    until "$@"; do
        tries=$((tries + 1))
        [ "$tries" -le 600 ] || return 1
        sleep 0.2
    done
}

# --- chain.sh: the wall, the date --------------------------------------------------------------------------------

scenario_chain() {
    echo "== chain.sh: the wall and the date"
    local w="$tmp/chain" n
    # A wall of 1.8 ms: the first game that the head searches for longer stops the chain.
    (
        export TAXMAN_WORK="$w" GAME_HOURS=0.0000005 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 1000 >"$tmp/chain.out" 2>&1
        echo $? >"$tmp/chain.status"
    )
    check "the wall ends the chain with status 4" has_line "$tmp/chain.status" 4
    check "the log says wall at N after H hours" matches "$w/logs/chain.log" 'wall at [0-9]* after [0-9.]* hours'
    check "the outcome says so" matches "$w/chain.outcome" '^wall at [0-9]* after [0-9.]* hours$'
    n="$(sed -n 's/^wall at \([0-9]*\) after.*/\1/p' "$w/chain.outcome")"
    check "the game has a checkpoint and a clock" exists "$w/ckpt/n$(printf %04d "$n").clock"
    check "and no line in the results" lacks "$w/results/strict.jsonl" "\"n\":$n,"
    # With a larger limit it goes on from its checkpoint and its clock, and completes.
    (
        export TAXMAN_WORK="$w" GAME_HOURS=1 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" $((n + 1)) >"$tmp/chain2.out" 2>&1
        echo $? >"$tmp/chain2.status"
    )
    check "a larger limit completes the chain" has_line "$tmp/chain2.status" 0
    check "and the clock is gone" absent "$w/ckpt/n$(printf %04d "$n").clock"
    check "and the game is in the results" matches "$w/results/strict.jsonl" "\"n\":$n,"

    # A date already within the margin: the deadline has passed before the solver starts.
    (
        export TAXMAN_WORK="$tmp/chain-date" RUN_UNTIL=$(($(date +%s) + 100)) THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 50 >"$tmp/chain3.out" 2>&1
        echo $? >"$tmp/chain3.status"
    )
    check "a date within the margin ends the chain with the deadline's status 3" has_line "$tmp/chain3.status" 3
    check "and the outcome names the date" matches "$tmp/chain-date/chain.outcome" '^deadline: the date 20'
}

# --- the ledger: the budget across machines ----------------------------------------------------------------------

scenario_ledger() {
    echo "== run.sh: the ledger in S3"
    local ledger
    ledger="$(obj t1/run/ledger)"
    : >"$FAKE_LOG"
    # The first machine, up half an hour: base 0.
    (
        machine 1800 boot-a
        export TAXMAN_WORK="$tmp/m1" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 60 >"$tmp/m1.out" 2>&1
        echo $? >"$tmp/m1.status"
    )
    check "the first machine completes the chain" has_line "$tmp/m1.status" 0
    check "the ledger has the run's start" matches "$ledger" '^start=[0-9]\{10\}$'
    check "and its boot" has_line "$ledger" boot=boot-a
    check "and base 0 and half an hour" has_line "$ledger" base=0
    check "and its uptime in hours" has_line "$ledger" hours=0.5000
    check "the results are in S3" matches "$(obj t1/results/strict.jsonl)" '"n":60,'
    local start
    start="$(sed -n 's/^start=//p' "$ledger")"

    # A replacement machine, up ten minutes: the first one's half hour is its base.
    (
        machine 600 boot-b
        export TAXMAN_WORK="$tmp/m2" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        mkdir -p "$tmp/m2"
        "$cloud/save.sh" --restore >/dev/null 2>&1
        "$cloud/chain.sh" 60 >"$tmp/m2.out" 2>&1
        echo $? >"$tmp/m2.status"
    )
    check "the replacement finds the chain done" has_line "$tmp/m2.status" 0
    check "the run's start is kept" has_line "$ledger" "start=$start"
    check "the replacement's base is the first machine's hours" has_line "$ledger" base=0.5000
    check "and the hours are the base and its own" has_line "$ledger" hours=0.6667

    # The same machine again (the same boot): its hours are not counted twice.
    (
        machine 3600 boot-b
        export TAXMAN_WORK="$tmp/m2" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 60 >"$tmp/m2b.out" 2>&1
    )
    check "a second script of the same boot keeps the base" has_line "$ledger" base=0.5000
    check "and the hours are base plus the boot's uptime" has_line "$ledger" hours=1.5000

    # A budget the earlier machines used up: nothing is started.
    (
        machine 600 boot-c
        export TAXMAN_WORK="$tmp/m3" S3_DEST=s3://b/t1 BUDGET_HOURS=1.5 THREADS=2 PROGRESS=0
        mkdir -p "$tmp/m3"
        "$cloud/save.sh" --restore >/dev/null 2>&1
        "$cloud/chain.sh" 60 >"$tmp/m3.out" 2>&1
        echo $? >"$tmp/m3.status"
    )
    check "a spent budget ends the chain with status 3" has_line "$tmp/m3.status" 3
    check "the outcome names the budget" matches "$tmp/m3/chain.outcome" '^deadline: the budget of 1.5 instance-hours$'
    check "the log says the deadline had passed" matches "$tmp/m3/logs/chain.log" \
        'the deadline has passed: the budget of 1.5 instance-hours'
    check "and the solver did not run" absent "$tmp/m3/logs/chain.out"

    # A ledger that cannot be read stops the machine: it must not run uncounted.
    (
        machine 600 boot-d
        export TAXMAN_WORK="$tmp/m4" S3_DEST=s3://b/t1 BUDGET_HOURS=10 FAKE_FAIL=nothing THREADS=2
        printf 'start=soon\nhours=lots\n' >"$ledger"
        "$cloud/chain.sh" 60 >"$tmp/m4.out" 2>&1
        echo $? >"$tmp/m4.status"
    )
    check "a ledger with no numbers stops the machine" has_line "$tmp/m4.status" 1
    check "and says so" matches "$tmp/m4.out" 'has no number for'
}

# --- wall-node.sh ------------------------------------------------------------------------------------------------

# A fresh node: its work directory, the config put in the fake S3 under t-NAME.
node_config() {
    local name="$1"
    shift
    mkdir -p "$(obj "$name/run")"
    printf '%s\n' "COMMIT=0123456789012345678901234567890123456789" "INSTANCE_TYPE=c8g.large" "WINDOW=2" "THREADS=2" \
        "CHECKPOINT_INTERVAL=2" "$@" >"$(obj "$name/run/config")"
}

# Runs a node for the run NAME in its own work directory, to its end.
node() {
    local name="$1"
    shift
    (
        export TAXMAN_WORK="$tmp/n-$name" S3_DEST="s3://b/$name" ASG=taxman-wall SKIP_SETUP=1 RETRY_SLEEP=0 \
            STOP_POLL=1 PROGRESS=0 "$@"
        machine 100 "boot-$name-$RANDOM"
        "$cloud/wall-node.sh" >"$tmp/n-$name.out" 2>&1
    )
}

scenario_node() {
    echo "== wall-node.sh"
    local name

    # A chain that completes: the run ends, the group goes to 0, the machine powers off.
    name=done
    node_config $name "CHAIN_TO=40" "BUDGET_HOURS=10" "RUN_UNTIL=$(($(date +%s) + 86400))"
    : >"$FAKE_LOG"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "a completed chain: the end is recorded" matches "$(obj $name/run/ended)" '^reason=complete: the range 1-40$'
    check "the group is taken to capacity 0" has_line "$FAKE_STATE/desired" 0
    check "the machine powers off" matches "$FAKE_LOG" '^shutdown -h now$'
    check "the machine recorded itself" exists "$(ls -d "$(obj $name/run/boots)"/* | head -n 1)"
    check "the results are in S3" matches "$(obj $name/results/strict.jsonl)" '"n":40,'
    check "the logs are in S3" exists "$(obj $name/logs/wall-node.log)"
    check "the ledger is there" matches "$(obj $name/run/ledger)" '^hours=0.0278$'

    # An ended run: a machine that starts later only takes the group to 0 again and powers off, with no chain.
    : >"$FAKE_LOG"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "an ended run is not taken up again" lacks "$tmp/n-$name.out" 'chain.sh [0-9]'
    check "but the group is taken to 0 again" has_line "$FAKE_STATE/desired" 0
    check "and the machine powers off" matches "$FAKE_LOG" '^shutdown -h now$'
    check "the first end stands" matches "$(obj $name/run/ended)" '^reason=complete: the range 1-40$'

    # The wall: a game over the limit ends the run with its reason, whatever the machine does next.
    name=wall
    node_config $name "CHAIN_TO=1000" "GAME_HOURS=0.0000005" "BUDGET_HOURS=10"
    : >"$FAKE_LOG"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "the wall: the end names the game" matches "$(obj $name/run/ended)" \
        '^reason=wall at [0-9]* after [0-9.]* hours$'
    check "the group is taken to 0" has_line "$FAKE_STATE/desired" 0
    check "the game's checkpoint and clock were saved to S3" lists "$(obj $name/ckpt)" 'clock$'

    # The budget: the chain is stopped by the deadline the ledger makes, and the end says the budget.
    name=budget
    node_config $name "CHAIN_TO=1000" "BUDGET_HOURS=0.0001" "LEDGER_MARGIN=0"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "the budget: the end names it" matches "$(obj $name/run/ended)" \
        '^reason=deadline: the budget of 0.0001 instance-hours$'

    # More machines than MAX_INSTANCES: a boot loop ends the run.
    name=loop
    node_config $name "CHAIN_TO=40" "MAX_INSTANCES=2"
    mkdir -p "$(obj $name/run/boots)"
    touch "$(obj $name/run/boots)/1" "$(obj $name/run/boots)/2"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "a third machine ends the run as a boot loop" matches "$(obj $name/run/ended)" 'MAX_INSTANCES=2'
    check "without a chain" absent "$tmp/n-$name/results/strict.jsonl"

    # A group that cannot be set to 0: the end is still recorded, the tries are made, the machine powers off.
    name=denied
    node_config $name "CHAIN_TO=20"
    : >"$FAKE_LOG"
    echo 1 >"$FAKE_STATE/desired"
    node $name FAKE_FAIL=set-desired-capacity
    check "a denied capacity: the end is recorded" matches "$(obj $name/run/ended)" '^reason=complete'
    check "five tries are made" equals "$(grep -c 'set-desired-capacity' "$FAKE_LOG")" 5
    check "and the machine powers off" matches "$FAKE_LOG" '^shutdown -h now$'

    # The setup is run as a build and no tests, with the pinned checkout left as it is.
    name=setup
    node_config $name "CHAIN_TO=20"
    echo 1 >"$FAKE_STATE/desired"
    node $name SKIP_SETUP=
    check "setup.sh is called with NO_PULL and SKIP_TESTS" matches "$tmp/n-$name/setup-stub.log" \
        'setup.sh: NO_PULL=1 SKIP_TESTS=1'
    check "and builds the solver alone" matches "$tmp/n-$name/setup-stub.log" 'BUILD=--bin taxman RELEASE_TESTS=$'

    # What the environment says about the build goes on to setup.sh; a setup that fails ends the run with the error.
    name=setupenv
    node_config $name "CHAIN_TO=20" "RELEASE_TESTS=1"
    echo 1 >"$FAKE_STATE/desired"
    node $name SKIP_SETUP= BUILD="--bin taxman --example ledger" STUB_SETUP_FAIL=1
    check "BUILD, and RELEASE_TESTS from the config, reach setup.sh" matches "$tmp/n-$name/setup-stub.log" \
        'BUILD=--bin taxman --example ledger RELEASE_TESTS=1$'
    check "a setup that fails ends the run" matches "$(obj $name/run/ended)" '^reason=setup.sh failed'
    check "and the chain does not start" absent "$tmp/n-$name/results/strict.jsonl"

    # A signal in the setup (the shutdown of a spot interruption sends SIGTERM to every process) is not the run's end:
    # no end is recorded, the group is left alone, and the machine does not power itself off.
    name=sigsetup
    node_config $name "CHAIN_TO=20"
    echo 1 >"$FAKE_STATE/desired"
    : >"$FAKE_LOG"
    node $name SKIP_SETUP= STUB_SETUP_SLEEP=30 &
    node_pid=$!
    wait_for matches "$tmp/n-$name/logs/wall-node.log" 'Z setup.sh$' || echo "the setup did not start"
    # As a shutdown does: the signal reaches the node and what it is running.
    local node_script
    node_script="$(pgrep -f "$cloud/wall-node.sh")"
    kill -TERM $node_script
    pkill -TERM -P "$node_script"
    wait "$node_pid" 2>/dev/null
    check "a signal in the setup does not end the run" absent "$(obj $name/run/ended)"
    check "and leaves the group alone" has_line "$FAKE_STATE/desired" 1
    check "and the machine does not power itself off" lacks "$FAKE_LOG" '^shutdown'
    check "the node says it was signalled" matches "$tmp/n-$name/logs/wall-node.log" 'signalled: leaving the run'

    # A node that cannot read its config ends the run with the error.
    name=noconfig
    mkdir -p "$(obj $name)"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "no config: the end is recorded" matches "$(obj $name/run/ended)" 'cannot read the run.s config'

    # The stop object: the chain is stopped cleanly and the run ends on request.
    name=stop
    node_config $name "CHAIN_TO=1300" "EXTRA_ARGS=--tactic gf2=off --tactic lagrange=on --tactic symmetry=off"
    echo 1 >"$FAKE_STATE/desired"
    node $name &
    local node_pid=$!
    wait_for exists "$tmp/n-$name/chain.pid" || echo "no chain.pid"
    sleep 3
    printf 'stop\n' >"$(obj $name/run/stop)"
    wait "$node_pid"
    check "a stop request ends the run" matches "$(obj $name/run/ended)" '^reason=stopped on request'
    check "the group is taken to 0" has_line "$FAKE_STATE/desired" 0
    check "the chain left a checkpoint" lists "$(obj $name/ckpt)" 'ckpt$'

    # An interruption: the chain is stopped by a signal and there is no stop object.  The machine waits to be taken
    # away; here it is (killed in its wait), and nothing has ended the run.
    name=interrupt
    node_config $name "CHAIN_TO=1300" "EXTRA_ARGS=--tactic gf2=off --tactic lagrange=on --tactic symmetry=off"
    echo 1 >"$FAKE_STATE/desired"
    node $name INTERRUPTION_WAIT=60 &
    node_pid=$!
    wait_for exists "$tmp/n-$name/chain.pid" || echo "no chain.pid"
    sleep 3
    kill -TERM "$(cat "$tmp/n-$name/chain.pid")"
    wait_for matches "$tmp/n-$name/logs/wall-node.log" 'an interruption, it seems' || echo "no interruption seen"
    pkill -P "$node_pid" -f 'sleep 60' 2>/dev/null
    kill -KILL "$node_pid" 2>/dev/null
    wait "$node_pid" 2>/dev/null
    check "an interruption does not end the run" absent "$(obj $name/run/ended)"
    check "and leaves the group alone" has_line "$FAKE_STATE/desired" 1
    check "and the chain's checkpoint is kept for the replacement" lists "$(obj $name/ckpt)" 'ckpt$'
    check "and its state is saved" exists "$(obj $name/results/strict.jsonl)"
}

# --- wall.sh -----------------------------------------------------------------------------------------------------

scenario_launch() {
    echo "== wall.sh"
    local commit=0123456789abcdef0123456789abcdef01234567 wall="$cloud/wall.sh"
    export BUCKET=taxman-013764204840-us-east-1 RUN=test WALL_DIR="$tmp/wall"
    # The object NAME of the run's bucket.
    wobj() { printf '%s/%s/%s\n' "$FAKE_S3" "$BUCKET" "$1"; }
    # The user data the template carries, decoded.
    template_user_data() {
        python3 -c 'import json, sys, base64
print(base64.b64decode(json.load(open(sys.argv[1]))["UserData"]).decode(), end="")' "$FAKE_STATE/template"
    }
    # The calls of the launch that change the account, in the order made, as `service command`.
    changes_in_order() {
        grep 'put-role-policy\|create-auto-scaling-group\|put-scheduled-update-group-action\|set-desired-capacity' \
            "$FAKE_LOG" | awk '{printf "%s %s,", $1, $2}'
    }
    rm -rf "$FAKE_STATE"
    : >"$FAKE_LOG"

    DRY_RUN=1 "$wall" launch --commit "$commit" --until 2099-01-01 >"$tmp/dry.out" 2>&1
    check "a dry run succeeds" equals $? 0
    check "and changes nothing" absent "$FAKE_STATE/asg"
    check "it prints the group's creation" matches "$tmp/dry.out" '^+ aws autoscaling create-auto-scaling-group'
    check "with capacity rebalancing off" matches "$tmp/dry.out" 'no-capacity-rebalance'
    check "it says it did not run it" matches "$tmp/dry.out" 'dry run: not run'
    check "the zones are by price, the cheapest first (b is 0.6603, a 0.6759, c has none)" matches "$tmp/dry.out" \
        'zones: us-east-1b us-east-1a us-east-1c'
    # The user data it prints is a script.
    sed -n '/^--- the user data of the launch template ---$/,/^--- the config of the run/p' "$tmp/dry.out" |
        sed '1d;$d' >"$tmp/user-data.sh"
    check "the user data is valid bash" bash -n "$tmp/user-data.sh"
    check "it checks out the commit" matches "$tmp/user-data.sh" "git -C /home/ec2-user/taxman checkout $commit"
    check "it starts wall-node.sh in tmux" matches "$tmp/user-data.sh" 'tmux new-session -d -s tx .*wall-node.sh'
    check "it sets the timer from the budget" matches "$tmp/user-data.sh" '^shutdown -h +4560$'
    check "the config has no build cache setting by default" lacks "$tmp/dry.out" '^RELEASE_TESTS='
    RELEASE_TESTS=1 REQUIRE_CACHED=1 DRY_RUN=1 "$wall" launch --commit "$commit" --until 2099-01-01 \
        >"$tmp/dry-cache.out" 2>&1
    check "RELEASE_TESTS and REQUIRE_CACHED go into the config" matches "$tmp/dry-cache.out" '^RELEASE_TESTS=1$'
    check "both" matches "$tmp/dry-cache.out" '^REQUIRE_CACHED=1$'

    "$wall" launch --commit "$commit" --until 2099-01-01 --zones "us-east-1b us-east-1a" >"$tmp/launch.out" 2>&1
    check "a launch succeeds" equals $? 0
    check "the group has capacity 1 at the end" has_line "$FAKE_STATE/desired" 1
    check "its mixed policy is spot, the lowest price" matches "$FAKE_STATE/asg" \
        '"SpotAllocationStrategy":"lowest-price"'
    check "the scheduled action is the date plus two hours" has_line "$FAKE_STATE/scheduled" 2099-01-01T02:00:00Z
    check "the template is JSON" python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$FAKE_STATE/template"
    check "the template's user data is the script" equals "$(template_user_data)" "$(cat "$tmp/user-data.sh")"
    check "the config is in S3" matches "$(wobj wall-test/run/config)" '^BUDGET_HOURS=74$'
    check "with the date as seconds" matches "$(wobj wall-test/run/config)" '^RUN_UNTIL=40[0-9]*$'
    check "the policy is on the role" exists "$FAKE_STATE/role-policy"
    check "the order: policy, group, schedule, start" equals "$(changes_in_order)" \
        "iam put-role-policy,autoscaling create-auto-scaling-group,autoscaling put-scheduled-update-group-action,autoscaling set-desired-capacity,"
    "$wall" launch --commit "$commit" --until 2099-01-01 >"$tmp/again.out" 2>&1
    check "a second launch is refused while the group exists" matches "$tmp/again.out" 'the group taxman-wall exists'
    "$wall" launch --commit abc --until 2099-01-01 >"$tmp/bad.out" 2>&1
    check "a short commit is refused" matches "$tmp/bad.out" '40 hex digits'

    "$wall" status >"$tmp/status.out" 2>&1
    check "status reads the run" matches "$tmp/status.out" 'budget 74 instance-hours'
    check "and says it has not ended" matches "$tmp/status.out" 'not ended'

    "$wall" stop >"$tmp/stop.out" 2>&1
    check "stop puts the stop object" exists "$(wobj wall-test/run/stop)"
    "$wall" stop --now >"$tmp/stopnow.out" 2>&1
    check "stop --now records the end" matches "$(wobj wall-test/run/ended)" 'stopped at once'
    check "and takes the group to 0" has_line "$FAKE_STATE/desired" 0
    "$wall" status >"$tmp/status2.out" 2>&1
    check "status shows the end" matches "$tmp/status2.out" 'ENDED .*stopped at once'

    "$wall" cleanup >"$tmp/cleanup.out" 2>&1
    check "cleanup succeeds on an ended run" equals $? 0
    check "the group is gone" absent "$FAKE_STATE/asg"
    check "the template is gone" absent "$FAKE_STATE/template"
    check "the security group is gone" absent "$FAKE_STATE/sg"
    check "the policy is gone" absent "$FAKE_STATE/role-policy"
    check "the run's state stays in S3" exists "$(wobj wall-test/run/config)"

    "$wall" launch --commit "$commit" --until 2099-01-01 >"$tmp/relaunch.out" 2>&1
    check "a launch over an existing run's state is refused" matches "$tmp/relaunch.out" 'run/config exists'
    mkdir -p "$(wobj wall-test/run/boots)"
    touch "$(wobj wall-test/run/boots)/1"
    "$wall" launch --commit "$commit" --until 2099-01-01 --continue >"$tmp/continue.out" 2>&1
    check "--continue goes on with it" equals $? 0
    check "and removes the end" absent "$(wobj wall-test/run/ended)"
    check "and the count of the machines so far" absent "$(wobj wall-test/run/boots)/1"
    check "and the stop" absent "$(wobj wall-test/run/stop)"
    "$wall" cleanup >"$tmp/cleanup2.out" 2>&1
    check "cleanup refuses a run that has not ended" matches "$tmp/cleanup2.out" 'has not ended'
    "$wall" cleanup --force >"$tmp/cleanup3.out" 2>&1
    check "cleanup --force does it" equals $? 0
}

# --- sweep.sh ----------------------------------------------------------------------------------------------------

# The lines of the file $1 for board size $2 (completed or not), one a line.
game_lines() {
    grep -F -- "{\"schema\":5,\"n\":$2," "$1"
}

# Whether the sweep of the merged file $1 over $2..$3 has nothing left to search again (sweep_plan.py's count is 0).
converged() {
    equals "$(python3 -I "$crate/tools/sweep_plan.py" excerpt "$1" "$2" "$3" /dev/null | awk '{print $2}')" 0
}

scenario_sweep() {
    echo "== sweep.sh: blocks, the passes over the merged file, resuming"
    local w="$tmp/sweep" res known pid
    # The real referees (verify.sh checks the merged file with both, and against the committed chain).
    cp "$real_cloud/verify.sh" "$cloud/verify.sh"
    : >"$FAKE_LOG"

    # 1301..1340 in four blocks and one second a game: the first block chains from the known games (1..1300), the other
    # blocks start standalone; the passes then search again the games that have more to give.
    known="$tmp/known1300.jsonl"
    head -n 1300 "$crate/results/strict.jsonl" >"$known"
    (
        export TAXMAN_WORK="$w" KNOWN="$known" BLOCKS=4 GAME_SECONDS=1 PROGRESS=0 MERGE_EVERY=5 S3_DEST=s3://b/sweep
        export SAVE_EVERY=3600
        "$cloud/sweep.sh" 1301 1340 >"$tmp/sweep.out" 2>&1
        echo $? >"$tmp/sweep.status"
    )
    res="$w/results/explore.jsonl"
    check "the sweep completes" has_line "$tmp/sweep.status" 0
    check "the outcome says so" matches "$w/sweep.outcome" '^complete: 1301\.\.1340 in 4 blocks'
    check "both referees accepted the merged file" matches "$tmp/sweep.out" 'both referees agree: no problem found'
    check "the plan is four blocks that tile 1301..1340" equals "$(awk '
        NR == 1 && $1 != 1301 {bad = 1} NR > 1 && $1 != last + 1 {bad = 1} {last = $2}
        END {print (NR == 4 && last == 1340 && !bad) ? "ok" : "bad"}' "$w/results/explore-blocks/plan")" ok
    check "the first block chains from the known games" matches "$res" '"n":1301,.*"prev":{"n":1300,'
    check "a block's first game has no game before it" matches "$res" '"n":1311,.*"prev":null'
    check "every game of the sweep says which pass made it" equals \
        "$(grep -E '^\{"schema":5,"n":(130[1-9]|13[1-3][0-9]|1340),' "$res" | grep -v -c '"pass":')" 0
    check "nothing is left that has more to give" converged "$res" 1301 1340
    check "the merged file is in S3" exists "$(obj sweep/results/explore.jsonl)"
    check "and is the one on the machine" cmp -s "$res" "$(obj sweep/results/explore.jsonl)"
    check "the blocks are in S3 too" exists "$(obj sweep/results/explore-blocks/block-004.jsonl)"

    # The same command again changes nothing.
    cp "$res" "$tmp/explore-before.jsonl"
    (
        export TAXMAN_WORK="$w" KNOWN="$known" BLOCKS=4 GAME_SECONDS=1 PROGRESS=0
        "$cloud/sweep.sh" 1301 1340 >"$tmp/sweep2.out" 2>&1
        echo $? >"$tmp/sweep2.status"
    )
    check "a rerun completes" has_line "$tmp/sweep2.status" 0
    check "and leaves the merged file as it was" cmp -s "$res" "$tmp/explore-before.jsonl"
    (
        export TAXMAN_WORK="$w" KNOWN="$known" GAME_SECONDS=1 PROGRESS=0
        "$cloud/sweep.sh" 1301 1341 >"$tmp/sweep3.out" 2>&1
    )
    check "another range over the same files is refused" matches "$tmp/sweep3.out" 'is the plan for 1301\.\.1340'

    # The boundary: block 2 begins at 1413, which standalone needs more than the limit and with the game before it, 1412,
    # proven (which a chain has) is settled at once by the ceiling.  Pass 2 searches it again with 1412, and the games
    # after it go on from it, as in an unbroken chain.
    # The known file here goes past FIRST - 1 (to 1420): the games of the range it holds are not taken.
    local w2="$tmp/sweep-boundary"
    head -n 1420 "$crate/results/strict.jsonl" >"$tmp/known1420.jsonl"
    mkdir -p "$w2/results/explore-blocks"
    printf '1411 1412\n1413 1416\n' >"$w2/results/explore-blocks/plan"
    (
        export TAXMAN_WORK="$w2" KNOWN="$tmp/known1420.jsonl" GAME_SECONDS=0.3 PROGRESS=0 MERGE_EVERY=5
        "$cloud/sweep.sh" 1411 1416 >"$tmp/boundary.out" 2>&1
        echo $? >"$tmp/boundary.status"
    )
    res="$w2/results/explore.jsonl"
    check "the boundary sweep completes, both referees agreeing" has_line "$tmp/boundary.status" 0
    check "the known games past 1410 are not taken: every line of 1411..1416 is the sweep's" equals \
        "$(grep -E '^\{"schema":5,"n":141[1-6],' "$res" | grep -v -c '"pass":')" 0
    check "1413 was unfinished in pass 1, with no game before it" matches "$res" \
        '"n":1413,"score":null,.*"prev":null.*"pass":1'
    check "and completed in pass 2 by the ceiling of 1412" matches "$res" \
        '"n":1413,"score":[0-9]*,.*"proof":"cap".*"prev":{"n":1412,.*"pass":2'
    check "1414 repeats the search of 1413" matches "$res" '"n":1414,"score":[0-9]*,.*"proof":"same_search".*"pass":2'
    check "1416 needs a search longer than the limit, as in a chain" matches "$res" '"n":1416,"score":null,.*"pass":2'
    check "nothing is left that has more to give at 1411..1416" converged "$res" 1411 1416

    # A SIGTERM stops it, with a line in the outcome and the games done kept; the same command goes on to the end.
    local w3="$tmp/sweep-stop"
    (
        export TAXMAN_WORK="$w3" KNOWN="$known" BLOCKS=4 GAME_SECONDS=1 PROGRESS=0 MERGE_EVERY=2
        "$cloud/sweep.sh" 1301 1340 >"$tmp/stop1.out" 2>&1
        echo $? >"$tmp/stop1.status"
    ) &
    pid=$!
    wait_for matches "$w3/results/explore-blocks/block-004.jsonl" '"n":1331,'
    kill -TERM "$(cat "$w3/sweep.pid")"
    wait "$pid"
    check "a SIGTERM stops the sweep with status 143" has_line "$tmp/stop1.status" 143
    check "and the outcome says stopped" matches "$w3/sweep.outcome" '^stopped by a signal$'
    check "the games done are in the merged file" matches "$w3/results/explore.jsonl" '"n":1331,'
    (
        export TAXMAN_WORK="$w3" KNOWN="$known" GAME_SECONDS=1 PROGRESS=0
        "$cloud/sweep.sh" 1301 1340 >"$tmp/stop2.out" 2>&1
        echo $? >"$tmp/stop2.status"
    )
    check "the same command completes it, both referees agreeing" has_line "$tmp/stop2.status" 0
    check "with every board size 1301..1340 either completed or unfinished, none missing" equals \
        "$(for n in $(seq 1301 1340); do game_lines "$w3/results/explore.jsonl" "$n" | head -n 1; done | wc -l | tr -d ' ')" 40
    check "and nothing left that has more to give" converged "$w3/results/explore.jsonl" 1301 1340
    cp "$tmp/verify.sh.stub" "$cloud/verify.sh"
}

selected=("$@")
[ "${#selected[@]}" -gt 0 ] || selected=(chain ledger node launch sweep)
for s in "${selected[@]}"; do
    case "$s" in
        chain) scenario_chain ;;
        ledger) scenario_ledger ;;
        node) scenario_node ;;
        launch) scenario_launch ;;
        sweep) scenario_sweep ;;
        *) echo "unknown scenario $s" >&2; exit 2 ;;
    esac
done
echo
if [ "$failures" = 0 ]; then echo "all checks passed"; else echo "$failures checks FAILED"; fi
exit "$failures"
