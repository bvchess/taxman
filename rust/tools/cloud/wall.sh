#!/usr/bin/env bash
# The unattended run to the wall: one spot instance in an Auto Scaling group that replaces it after an interruption,
# running the chain from n = 1 until a game's own search passes the wall, the budget of instance-hours is used, the date
# comes, or something fails.  This is the script for your machine; the instance's side is wall-node.sh (RUNBOOK.md,
# section 12).
#
#   rust/tools/cloud/wall.sh launch --commit SHA --until DATE [options]   create everything and start the run
#   rust/tools/cloud/wall.sh status                                        where the run is, from S3 and the group
#   rust/tools/cloud/wall.sh stop [--now]                                  end the run (cleanly, or at once)
#   rust/tools/cloud/wall.sh cleanup [--force]                             delete what launch made; check what is left
#
# Every aws command is printed, as `+ aws ...`, before it runs (reads and changes alike); DRY_RUN=1 runs the reads and
# prints, without running, the commands that create, change or delete something.  `launch` does, in this order:
#
#   1. writes $S3_DEST/run/config, the run's settings (the commit, the budget, the date, the wall, ...), which every
#      machine reads
#   2. puts the inline policy `taxman-wall` on the role taxman-ec2 (wall-policy.json): a machine may set the group's
#      desired capacity (to 0, at the end of the run) and read the group
#   3. creates a security group `taxman-wall` with no inbound rule, and 4. the launch template `taxman-wall`, whose user
#      data installs, checks out the commit, and starts wall-node.sh in tmux
#   5. creates the Auto Scaling group `taxman-wall` (one spot instance of the type, the cheapest zone first, capacity
#      rebalancing off) at desired capacity 0, 6. puts a scheduled action that takes it to 0 at the date plus two hours,
#      whatever the machines do, and 7. sets desired capacity 1: the first machine starts
#
# launch options (each also an environment variable):
#   --commit SHA          COMMIT          the commit the machines check out (40 hex digits; pushed to origin)
#   --until DATE          UNTIL           the hard date (UTC): YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
#   --budget-hours H      BUDGET_HOURS    instance-hours in all, over every machine                   default 74
#   --game-hours H        GAME_HOURS      the wall: one game's own search, in hours                   default 24
#   --chain-to N          CHAIN_TO        the last board size                                         default 8191
#   --window K            WINDOW          games in flight                                             default 4
#   --threads K           THREADS         worker threads                                              default all
#   --instance-type T     INSTANCE_TYPE   the type of the one instance                              default c8g.16xlarge
#   --zones "Z1 Z2"       ZONES           availability zones, cheapest first                          default every zone
#                                         that offers the type, by the spot price of the last day
#   --spot-max P          SPOT_MAX        the most an hour of spot may cost, dollars                  default 1.0
#   --price P             PRICE           what an instance-hour is counted at in `status`, dollars    default 0.676
#   --volume-gb G         VOLUME_GB       the root volume                                             default 40
#   --extra-args "..."    EXTRA_ARGS      more flags for `taxman solve`
#   --margin S            LEDGER_MARGIN   seconds before the budget or the date that the chain stops  default 300
#   --max-instances N     MAX_INSTANCES   machines in all before the run ends as a boot loop         default 20
#   --save-every S        SAVE_EVERY      seconds between the copies to S3 (run.sh)                  default 600
#   --checkpoint-interval S               seconds between checkpoints (CHECKPOINT_INTERVAL, chain.sh) default 300
#   --continue                            go on with a run that has ended or been stopped (its S3 state is kept; the
#                                         ended and stop objects are removed, the config is written again)
# The build cache (RUNBOOK.md, section 15), by environment variables only; the config carries them to each machine's
# setup.sh: RELEASE_TESTS=1 (a machine whose cache has no tested build of the commit builds it, runs the release suite
# and, if it passes, stores the binaries; if it fails the run ends with RELEASE_TESTS_FAILED in $S3_DEST),
# REQUIRE_CACHED=1 (a machine whose cache has none ends the run instead of building).
# Other environment: RUN (the name of the run; its state is $S3_DEST = s3://BUCKET/wall-RUN; default run1), BUCKET,
# AWS_PROFILE and AWS_DEFAULT_REGION (us-east-1), DRY_RUN, AWS_CLI, and WALL_DIR (default ~/taxman-wall/RUN): where the
# files that the commands take are written and kept (the user data, the launch template's data, the config).  The
# group, template, policy and security group are always named taxman-wall: there is one run at a time.

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

AWS_CLI="${AWS_CLI:-aws}"
export AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-us-east-1}"
BUCKET="${BUCKET:-taxman-013764204840-us-east-1}"
RUN="${RUN:-run1}"
NAME=taxman-wall
ROLE=taxman-ec2
S3_DEST="s3://$BUCKET/wall-$RUN"
DRY_RUN="${DRY_RUN:-0}"
TAGS='{Key=Project,Value=taxman},{Key=Purpose,Value=wall}'
IMAGE="resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64"

# The files the commands take (the user data, the template, the config, ...) are written here and kept, so that a
# command that was printed can be run again by hand.
work="${WALL_DIR:-$HOME/taxman-wall/$RUN}"

# --- aws, printed ------------------------------------------------------------------------------------------------

# Prints a command as the shell would take it back: an argument with anything but plain characters is in single quotes.
show() {
    local arg out="+ aws"
    for arg in "$@"; do
        case "$arg" in
            '' | *[!A-Za-z0-9_./:=@%+,-]*) out="$out '$(printf '%s' "$arg" | sed "s/'/'\\\\''/g")'" ;;
            *) out="$out $arg" ;;
        esac
    done
    printf '%s\n' "$out" >&2
}

# A command that only reads.
read_aws() {
    show "$@"
    "$AWS_CLI" "$@"
}

# A command that creates, changes or deletes something: not run when DRY_RUN=1.
change_aws() {
    show "$@"
    if [ "$DRY_RUN" = 1 ]; then
        echo "  (dry run: not run)" >&2
        return 0
    fi
    "$AWS_CLI" "$@"
}

# change_aws for a command whose output is wanted: the first argument is what a dry run answers in its place.
change_aws_value() {
    local placeholder="$1"
    shift
    if [ "$DRY_RUN" = 1 ]; then
        show "$@"
        echo "  (dry run: not run)" >&2
        echo "$placeholder"
        return 0
    fi
    change_aws "$@"
}

# The object NAME of S3_DEST/run, or nothing (a read that is not printed: status asks for several).
run_object() {
    "$AWS_CLI" s3 cp "$S3_DEST/run/$1" - --only-show-errors 2>/dev/null
}

# Whether the object NAME is in S3_DEST/run.
has_object() {
    local listing
    show s3 ls "$S3_DEST/run/$1"
    listing="$("$AWS_CLI" s3 ls "$S3_DEST/run/$1" 2>/dev/null)"
    printf '%s\n' "$listing" | grep -q " $1\$"
}

# The value of NAME in the text on stdin (a config or a ledger).
field() {
    sed -n "s/^$1=//p" | tail -n 1
}

# --- helpers -----------------------------------------------------------------------------------------------------

# The seconds since the epoch of a UTC date, YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ (GNU date, then BSD date).
to_epoch() {
    local text="$1"
    case "$text" in
        ????-??-??) text="${text}T00:00:00Z" ;;
        ????-??-??T??:??:??Z) ;;
        *) return 1 ;;
    esac
    date -u -d "$text" +%s 2>/dev/null || date -u -j -f '%Y-%m-%dT%H:%M:%SZ' "$text" +%s 2>/dev/null
}

# The comment at the top of this file.
usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

is_number() {
    case "$1" in '' | *[!0-9.]* | *.*.*) return 1 ;; esac
    return 0
}

need_account() {
    local account
    account="$(read_aws sts get-caller-identity --query Account --output text)" ||
        die "no AWS credentials (aws login --profile bvc, then export AWS_PROFILE=bvc)"
    case "$BUCKET" in
        *"$account"*) log "account $account, region $AWS_DEFAULT_REGION, run $RUN: $S3_DEST" ;;
        *) die "the bucket $BUCKET does not carry the account number $account: wrong profile?" ;;
    esac
}

# --- launch ------------------------------------------------------------------------------------------------------

# The zones in which the type is offered, the cheapest spot price of the last day first.
zones_by_price() {
    local type="$1" offered prices zone
    offered="$(read_aws ec2 describe-instance-type-offerings --location-type availability-zone \
        --filters "Name=instance-type,Values=$type" --query 'InstanceTypeOfferings[].Location' --output text)" ||
        die "cannot read where $type is offered"
    prices="$(read_aws ec2 describe-spot-price-history --instance-types "$type" --product-descriptions Linux/UNIX \
        --start-time "$(epoch_date $(($(date +%s) - 86400)))" \
        --query 'SpotPriceHistory[].[Timestamp,AvailabilityZone,SpotPrice]' --output text)" ||
        die "cannot read the spot price history of $type"
    # The latest price of each zone (the history is newest first), then the zones by price; a zone with none, last.
    for zone in $offered; do
        printf '%s %s\n' "$(printf '%s\n' "$prices" | sort -r | awk -v z="$zone" '$2 == z {print $3; exit}')" "$zone"
    done | awk '{if (NF == 1) print "99", $1; else print}' | sort -n
}

# The user data of the launch template: a script that sets the timer, installs, checks out the commit and starts
# wall-node.sh in tmux.  If any of that fails it ends the run with the reason, so that the group does not launch the
# same failure again and again.
write_user_data() {
    local file="$1" timer_minutes="$2" commit="$3"
    {
        printf '#!/bin/bash\n'
        printf '# The user data of the launch template %s (rust/tools/cloud/wall.sh).\n' "$NAME"
        printf 'export AWS_DEFAULT_REGION=%s S3_DEST=%s ASG=%s\n' "$AWS_DEFAULT_REGION" "$S3_DEST" "$NAME"
        printf '# The timer: no machine lives longer than the whole budget and two hours, whatever the scripts do.\n'
        printf 'shutdown -h +%s\n' "$timer_minutes"
        printf 'retry() { local n=0; until "$@"; do n=$((n + 1)); [ "$n" -lt 5 ] || return 1; sleep 15; done; }\n'
        printf 'fail() {\n'
        printf '    echo "boot failed: $*"\n'
        printf '    printf "time=%%s\\nreason=%%s\\n" "$(date +%%s)" "boot failed: $*" |\n'
        printf '        aws s3 cp - "$S3_DEST/run/ended"\n'
        printf '    aws autoscaling set-desired-capacity --auto-scaling-group-name "$ASG" --desired-capacity 0\n'
        printf '    shutdown -h now\n'
        printf '    exit 1\n'
        printf '}\n'
        printf 'clone() {\n'
        printf '    rm -rf /home/ec2-user/taxman\n'
        printf '    sudo -u ec2-user git clone https://github.com/bvchess/taxman.git /home/ec2-user/taxman\n'
        printf '}\n'
        printf 'retry dnf install -y git tmux || fail "dnf install git tmux"\n'
        printf 'retry clone || fail "git clone"\n'
        printf 'sudo -u ec2-user git -C /home/ec2-user/taxman checkout %s ||\n' "$commit"
        printf '    fail "git checkout %s"\n' "$commit"
        printf 'sudo -u ec2-user -i env AWS_DEFAULT_REGION=%s S3_DEST=%s ASG=%s \\\n' "$AWS_DEFAULT_REGION" "$S3_DEST" \
            "$NAME"
        printf '    tmux new-session -d -s tx -c /home/ec2-user/taxman rust/tools/cloud/wall-node.sh ||\n'
        printf '    fail "tmux"\n'
    } >"$file"
}

# The launch template's data, as JSON.
write_template_data() {
    local file="$1" user_data="$2" type="$3" sg="$4" volume="$5"
    {
        printf '{"ImageId":"%s","InstanceType":"%s","IamInstanceProfile":{"Name":"%s"},"SecurityGroupIds":["%s"],' \
            "$IMAGE" "$type" "$ROLE" "$sg"
        printf '"MetadataOptions":{"HttpTokens":"required","HttpEndpoint":"enabled"},'
        printf '"BlockDeviceMappings":[{"DeviceName":"/dev/xvda","Ebs":{"VolumeSize":%s,"VolumeType":"gp3",' "$volume"
        printf '"DeleteOnTermination":true}}],"InstanceInitiatedShutdownBehavior":"terminate","UserData":"%s",' \
            "$(base64 <"$user_data" | tr -d '\n')"
        printf '"TagSpecifications":[{"ResourceType":"instance","Tags":[{"Key":"Name","Value":"%s"},' "$NAME"
        printf '{"Key":"Project","Value":"taxman"},{"Key":"Purpose","Value":"wall"}]},'
        printf '{"ResourceType":"volume","Tags":[{"Key":"Project","Value":"taxman"},'
        printf '{"Key":"Purpose","Value":"wall"}]}]}\n'
    } >"$file"
}

# The group's mixed instances policy: spot only, the lowest price of the zones, one instance type.
write_mixed_policy() {
    local file="$1" type="$2" spot_max="$3"
    {
        printf '{"LaunchTemplate":{"LaunchTemplateSpecification":{"LaunchTemplateName":"%s",' "$NAME"
        printf '"Version":"$Latest"},'
        printf '"Overrides":[{"InstanceType":"%s"}]},"InstancesDistribution":{"OnDemandBaseCapacity":0,' "$type"
        printf '"OnDemandPercentageAboveBaseCapacity":0,"SpotAllocationStrategy":"lowest-price","SpotInstancePools":1,'
        printf '"SpotMaxPrice":"%s"}}\n' "$spot_max"
    } >"$file"
}

# The config object: name=value lines, the settings that were given (wall-node.sh takes the names it knows, each as the
# rest of its line; PRICE is for status).
write_config() {
    local file="$1" name
    : >"$file"
    for name in COMMIT INSTANCE_TYPE CHAIN_TO WINDOW GAME_HOURS BUDGET_HOURS THREADS EXTRA_ARGS CHECKPOINT_INTERVAL \
        SAVE_EVERY LEDGER_MARGIN MAX_INSTANCES PRICE RELEASE_TESTS REQUIRE_CACHED; do
        [ -z "${!name}" ] || printf '%s=%s\n' "$name" "${!name}" >>"$file"
    done
    printf 'RUN_UNTIL=%s\n' "$UNTIL_EPOCH" >>"$file"
}

launch() {
    COMMIT="${COMMIT:-}"
    UNTIL="${UNTIL:-}"
    BUDGET_HOURS="${BUDGET_HOURS:-74}"
    GAME_HOURS="${GAME_HOURS:-24}"
    CHAIN_TO="${CHAIN_TO:-8191}"
    WINDOW="${WINDOW:-4}"
    THREADS="${THREADS:-}"
    INSTANCE_TYPE="${INSTANCE_TYPE:-c8g.16xlarge}"
    ZONES="${ZONES:-}"
    SPOT_MAX="${SPOT_MAX:-1.0}"
    PRICE="${PRICE:-0.676}"
    VOLUME_GB="${VOLUME_GB:-40}"
    EXTRA_ARGS="${EXTRA_ARGS:-}"
    LEDGER_MARGIN="${LEDGER_MARGIN:-300}"
    MAX_INSTANCES="${MAX_INSTANCES:-20}"
    SAVE_EVERY="${SAVE_EVERY:-}"
    CHECKPOINT_INTERVAL="${CHECKPOINT_INTERVAL:-}"
    RELEASE_TESTS="${RELEASE_TESTS:-}"
    REQUIRE_CACHED="${REQUIRE_CACHED:-}"
    local continuing=0 number
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --continue)
                continuing=1
                shift
                continue
                ;;
            -h | --help)
                usage
                exit 0
                ;;
        esac
        [ "$#" -ge 2 ] || die "launch: $1 needs a value (see the top of $0)"
        case "$1" in
            --commit) COMMIT="$2" ;;
            --until) UNTIL="$2" ;;
            --budget-hours) BUDGET_HOURS="$2" ;;
            --game-hours) GAME_HOURS="$2" ;;
            --chain-to) CHAIN_TO="$2" ;;
            --window) WINDOW="$2" ;;
            --threads) THREADS="$2" ;;
            --instance-type) INSTANCE_TYPE="$2" ;;
            --zones) ZONES="$2" ;;
            --spot-max) SPOT_MAX="$2" ;;
            --price) PRICE="$2" ;;
            --volume-gb) VOLUME_GB="$2" ;;
            --extra-args) EXTRA_ARGS="$2" ;;
            --margin) LEDGER_MARGIN="$2" ;;
            --max-instances) MAX_INSTANCES="$2" ;;
            --save-every) SAVE_EVERY="$2" ;;
            --checkpoint-interval) CHECKPOINT_INTERVAL="$2" ;;
            *) die "launch: unknown option $1 (see the top of $0)" ;;
        esac
        shift 2
    done
    if [ "${#COMMIT}" != 40 ] || [ -n "$(printf '%s' "$COMMIT" | tr -d '0-9a-f')" ]; then
        die "launch: --commit must be a full commit id of 40 hex digits (git rev-parse), pushed to origin"
    fi
    [ -n "$UNTIL" ] || die "launch: --until DATE is needed: the hard date after which the run stops whatever its state"
    UNTIL_EPOCH="$(to_epoch "$UNTIL")" || die "launch: cannot read the date $UNTIL (YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ)"
    [ "$UNTIL_EPOCH" -gt "$(date +%s)" ] || die "launch: the date $UNTIL is past"
    for number in "$BUDGET_HOURS" "$GAME_HOURS" "$SPOT_MAX" "$PRICE" "$LEDGER_MARGIN" "$MAX_INSTANCES" "$VOLUME_GB" \
        "$CHAIN_TO" "$WINDOW"; do
        is_number "$number" || die "launch: \"$number\" is not a number"
    done
    for number in "$SAVE_EVERY" "$CHECKPOINT_INTERVAL"; do
        [ -z "$number" ] || is_number "$number" || die "launch: \"$number\" is not a number"
    done
    [ -r "$here/wall-policy.json" ] || die "launch: $here/wall-policy.json is missing"
    if git -C "$here" cat-file -e "$COMMIT^{commit}" 2>/dev/null; then
        [ -n "$(git -C "$here" branch -r --contains "$COMMIT" 2>/dev/null)" ] ||
            log "warning: $COMMIT is on no remote branch of this checkout (git fetch? git push?):" \
                "the machines clone it from GitHub"
    else
        log "warning: $COMMIT is unknown to this checkout: the machines clone it from GitHub"
    fi

    need_account
    mkdir -p "$work"

    # What exists already: a run is one at a time.
    local found
    found="$(read_aws autoscaling describe-auto-scaling-groups --auto-scaling-group-names "$NAME" \
        --query 'AutoScalingGroups[].AutoScalingGroupName' --output text)" || die "cannot read the groups"
    [ -z "$found" ] || die "the group $NAME exists: wall.sh cleanup first (or wall.sh stop, status)"
    found="$(read_aws ec2 describe-launch-templates --filters "Name=launch-template-name,Values=$NAME" \
        --query 'LaunchTemplates[].LaunchTemplateName' --output text)" || die "cannot read the launch templates"
    [ -z "$found" ] || die "the launch template $NAME exists: wall.sh cleanup first"
    if has_object config; then
        if [ "$continuing" = 1 ]; then
            log "going on with the run whose state is in $S3_DEST (its ledger, results and checkpoints are kept)"
        else
            die "$S3_DEST/run/config exists: that run's state is there; use another RUN, or --continue to go on with it"
        fi
    elif [ "$continuing" = 1 ]; then
        die "--continue, but $S3_DEST/run/config does not exist: there is no run to go on with"
    fi
    read_aws iam get-instance-profile --instance-profile-name "$ROLE" --query 'InstanceProfile.Roles[].RoleName' \
        --output text >/dev/null || die "the instance profile $ROLE does not exist (RUNBOOK.md, section 9.4)"

    # Where: the zones, cheapest first, and their subnets in the default VPC.
    local vpc subnets zone subnet
    if [ -z "$ZONES" ]; then
        local ranked
        ranked="$(zones_by_price "$INSTANCE_TYPE")" || exit 1
        log "zones by the spot price of the last day (dollars an hour, zone):"
        printf '%s\n' "$ranked" | sed 's/^/    /' >&2
        ZONES="$(printf '%s\n' "$ranked" | awk '{print $2}' | tr '\n' ' ')"
    fi
    vpc="$(read_aws ec2 describe-vpcs --filters Name=is-default,Values=true --query 'Vpcs[0].VpcId' --output text)" ||
        die "cannot read the default VPC"
    subnets=""
    for zone in $ZONES; do
        subnet="$(read_aws ec2 describe-subnets \
            --filters "Name=vpc-id,Values=$vpc" "Name=availability-zone,Values=$zone" Name=default-for-az,Values=true \
            --query 'Subnets[0].SubnetId' --output text)" || die "cannot read the subnet of $zone"
        { [ -n "$subnet" ] && [ "$subnet" != None ]; } || die "no default subnet in $zone"
        subnets="${subnets:+$subnets,}$subnet"
    done
    [ -n "$subnets" ] || die "no zone for $INSTANCE_TYPE"

    # The files the commands take, shown before they are used.
    local timer_minutes
    timer_minutes="$(awk -v h="$BUDGET_HOURS" 'BEGIN {printf "%d", (h + 2) * 60}')"
    write_user_data "$work/user-data.sh" "$timer_minutes" "$COMMIT"
    write_config "$work/config"
    write_mixed_policy "$work/mixed.json" "$INSTANCE_TYPE" "$SPOT_MAX"
    printf '\n--- the user data of the launch template ---\n' >&2
    cat "$work/user-data.sh" >&2
    printf -- '--- the config of the run, %s ---\n' "$S3_DEST/run/config" >&2
    cat "$work/config" >&2
    printf -- '--- the policy for the role %s (%s) ---\n' "$ROLE" "$here/wall-policy.json" >&2
    cat "$here/wall-policy.json" >&2
    printf -- '--- the mixed instances policy of the group ---\n' >&2
    cat "$work/mixed.json" >&2
    printf -- '---\n\n' >&2
    local dollars
    dollars="$(awk -v h="$BUDGET_HOURS" -v p="$PRICE" 'BEGIN {printf "%.2f", h * p}')"
    log "budget $BUDGET_HOURS instance-hours (about \$$dollars at \$$PRICE an hour), wall $GAME_HOURS hours," \
        "until $UNTIL, zones: $ZONES"

    # 1. The run's config (going on, with no end and no stop).
    if [ "$continuing" = 1 ]; then
        has_object ended && change_aws s3 rm "$S3_DEST/run/ended"
        has_object stop && change_aws s3 rm "$S3_DEST/run/stop"
        # The machines of the run so far are not a boot loop of this one.
        change_aws s3 rm "$S3_DEST/run/boots/" --recursive --only-show-errors
    fi
    change_aws s3 cp "$work/config" "$S3_DEST/run/config" --only-show-errors || die "cannot write the config"
    # 2. The permission the machines need at the end.
    change_aws iam put-role-policy --role-name "$ROLE" --policy-name "$NAME" \
        --policy-document "file://$here/wall-policy.json" || die "cannot put the policy"
    # 3. The security group.
    local sg
    sg="$(change_aws_value sg-DRYRUN ec2 create-security-group --group-name "$NAME" --vpc-id "$vpc" \
        --description 'taxman wall run, no inbound' --tag-specifications "ResourceType=security-group,Tags=[$TAGS]" \
        --query GroupId --output text)" || die "cannot create the security group"
    # 4. The launch template.
    write_template_data "$work/template.json" "$work/user-data.sh" "$INSTANCE_TYPE" "$sg" "$VOLUME_GB"
    change_aws ec2 create-launch-template --launch-template-name "$NAME" \
        --version-description "wall run $RUN, commit $COMMIT" --launch-template-data "file://$work/template.json" \
        --tag-specifications "ResourceType=launch-template,Tags=[$TAGS]" --query LaunchTemplate.LaunchTemplateId \
        --output text || die "cannot create the launch template (wall.sh cleanup --force removes what was made)"
    # 5. The group, at capacity 0 for now: one instance at most, no capacity rebalancing, EC2 health checks.
    change_aws autoscaling create-auto-scaling-group --auto-scaling-group-name "$NAME" \
        --mixed-instances-policy "file://$work/mixed.json" --min-size 0 --max-size 1 --desired-capacity 0 \
        --vpc-zone-identifier "$subnets" --no-capacity-rebalance --health-check-type EC2 --health-check-grace-period 0 \
        --tags "ResourceId=$NAME,ResourceType=auto-scaling-group,Key=Project,Value=taxman,PropagateAtLaunch=true" \
        "ResourceId=$NAME,ResourceType=auto-scaling-group,Key=Purpose,Value=wall,PropagateAtLaunch=true" \
        "ResourceId=$NAME,ResourceType=auto-scaling-group,Key=Name,Value=$NAME,PropagateAtLaunch=true" ||
        die "cannot create the group (wall.sh cleanup --force removes what was made)"
    # 6. The backstop that depends on no script: at the date and two hours, capacity 0.
    change_aws autoscaling put-scheduled-update-group-action --auto-scaling-group-name "$NAME" \
        --scheduled-action-name "$NAME-date" --start-time "$(epoch_date $((UNTIL_EPOCH + 7200)))" \
        --min-size 0 --max-size 0 --desired-capacity 0 ||
        die "cannot put the scheduled action (wall.sh cleanup --force removes what was made)"
    # 7. Go.
    change_aws autoscaling set-desired-capacity --auto-scaling-group-name "$NAME" --desired-capacity 1 ||
        die "cannot start the first machine"
    printf '%s\n' "launched.  The first machine starts in a minute or two and builds for some minutes.  Then:" \
        "    $0 status          where the run is" "    $0 stop [--now]    end it" \
        "    $0 cleanup         delete what launch made, once the run has ended" >&2
}

# --- status ------------------------------------------------------------------------------------------------------

status() {
    need_account
    local config ended ledger hours start updated budget price until_epoch boots
    config="$(run_object config)" || die "no run $RUN in $S3_DEST (nothing at $S3_DEST/run/config)"
    budget="$(printf '%s\n' "$config" | field BUDGET_HOURS)"
    price="$(printf '%s\n' "$config" | field PRICE)"
    until_epoch="$(printf '%s\n' "$config" | field RUN_UNTIL)"
    printf 'run %s  (%s)\n' "$RUN" "$S3_DEST"
    printf '  commit %s, %s, wall %s hours, budget %s instance-hours, until %s\n' \
        "$(printf '%s\n' "$config" | field COMMIT | cut -c1-12)" "$(printf '%s\n' "$config" | field INSTANCE_TYPE)" \
        "$(printf '%s\n' "$config" | field GAME_HOURS)" "$budget" "$(epoch_date "$until_epoch")"

    ended="$(run_object ended)"
    if [ -n "$ended" ]; then
        printf '  ENDED %s: %s\n' "$(epoch_date "$(printf '%s\n' "$ended" | field time)")" \
            "$(printf '%s\n' "$ended" | field reason)"
    elif has_object stop; then
        printf '  stop requested: the machine is stopping the chain, then ends the run\n'
    else
        printf '  not ended\n'
    fi

    ledger="$(run_object ledger)"
    if [ -n "$ledger" ]; then
        start="$(printf '%s\n' "$ledger" | field start)"
        hours="$(printf '%s\n' "$ledger" | field hours)"
        updated="$(printf '%s\n' "$ledger" | field updated)"
        awk -v start="$start" -v now="$(date +%s)" -v hours="$hours" -v budget="$budget" -v price="$price" \
            -v began="$(epoch_date "$start")" -v seen="$(epoch_date "$updated")" 'BEGIN {
            printf "  the run began %s, %.2f hours of wall clock ago\n", began, (now - start) / 3600
            printf "  instance-hours %.2f of %s (%.0f%%), counted to %s\n", hours, budget, 100 * hours / budget, seen
            printf "  spend about $%.2f of $%.2f, at $%s an hour\n", hours * price, budget * price, price }'
    else
        printf '  no ledger yet: no machine has started the chain\n'
    fi
    boots="$(read_aws s3 ls "$S3_DEST/run/boots/" 2>/dev/null | wc -l | tr -d ' ')"
    printf '  machines started so far: %s\n' "$boots"

    printf 'the group %s\n' "$NAME"
    read_aws autoscaling describe-auto-scaling-groups --auto-scaling-group-names "$NAME" --output text \
        --query 'AutoScalingGroups[0].[DesiredCapacity,Instances[].[InstanceId,AvailabilityZone,LifecycleState]]' \
        2>&1 | sed 's/^/  /'

    printf 'the chain (as of the last save to S3)\n'
    local out clocks clock hours_held
    out="$("$AWS_CLI" s3 cp "$S3_DEST/logs/chain.out" - --only-show-errors 2>/dev/null | grep '^n=' | tail -n 1)"
    printf '  last game done: %s\n' "${out:-none yet}"
    out="$("$AWS_CLI" s3 cp "$S3_DEST/logs/chain.err" - --only-show-errors 2>/dev/null | tail -n 1 | cut -c1-200)"
    printf '  latest progress line: %s\n' "$out"
    clocks="$("$AWS_CLI" s3 ls "$S3_DEST/ckpt/" 2>/dev/null | awk '$4 ~ /\.clock$/ {print $4}')"
    for clock in $clocks; do
        hours_held="$("$AWS_CLI" s3 cp "$S3_DEST/ckpt/$clock" - --only-show-errors 2>/dev/null |
            awk '{printf "%.2f", $1 / 3600}')"
        printf '  wall clock of %s: %s of %s hours\n' "${clock%.clock}" "$hours_held" \
            "$(printf '%s\n' "$config" | field GAME_HOURS)"
    done
    out="$("$AWS_CLI" s3 ls "$S3_DEST/results/strict.jsonl" 2>/dev/null | awk '{print $1, $2, "UTC"}')"
    printf '  last save of the results: %s\n' "$out"
}

# --- stop --------------------------------------------------------------------------------------------------------

stop() {
    local now=0
    [ "${1:-}" = --now ] && now=1
    need_account
    mkdir -p "$work"
    if [ "$now" = 1 ]; then
        # The end is written first, so that no machine that starts meanwhile takes up the run.
        printf 'time=%s\nreason=%s\n' "$(date +%s)" "stopped at once by wall.sh stop --now" >"$work/ended"
        has_object ended || change_aws s3 cp "$work/ended" "$S3_DEST/run/ended" --only-show-errors ||
            die "cannot write the end"
        change_aws autoscaling set-desired-capacity --auto-scaling-group-name "$NAME" --desired-capacity 0 ||
            die "cannot set the desired capacity"
        log "the machine is being terminated: what it did since its last save (at most SAVE_EVERY seconds) is lost"
    else
        printf '%s\n' "stop" >"$work/stop"
        change_aws s3 cp "$work/stop" "$S3_DEST/run/stop" --only-show-errors || die "cannot write the stop object"
        log "the machine looks for it every 30 seconds, stops the chain with a final checkpoint, saves, records the" \
            "end, takes the group to capacity 0 and powers off; wall.sh status shows when"
    fi
}

# --- cleanup -----------------------------------------------------------------------------------------------------

# The instances of the project that are not gone, as arguments of aws.
live_instances=(ec2 describe-instances --filters Name=tag:Project,Values=taxman
    Name=instance-state-name,Values=pending,running,shutting-down,stopping,stopped
    --query 'Reservations[].Instances[].InstanceId')

# Waits until no instance of the project is left (for the group's machines to be gone), at most ten minutes.
wait_for_no_instances() {
    local tries=0 left
    show "${live_instances[@]}" --output text
    while :; do
        left="$("$AWS_CLI" "${live_instances[@]}" --output text)" || die "cannot read the instances"
        [ -z "$left" ] && return 0
        tries=$((tries + 1))
        [ "$tries" -le 40 ] || die "still there after ten minutes: $left"
        printf '  waiting for %s ...\n' "$left" >&2
        sleep 15
    done
}

cleanup() {
    local force=0
    [ "${1:-}" = --force ] && force=1
    need_account
    if [ "$force" != 1 ] && ! has_object ended; then
        die "the run has not ended ($S3_DEST/run/ended is not there): wall.sh stop first, or cleanup --force"
    fi
    local sg tries
    change_aws autoscaling delete-auto-scaling-group --auto-scaling-group-name "$NAME" --force-delete ||
        log "no group, or not deleted"
    [ "$DRY_RUN" = 1 ] || wait_for_no_instances
    change_aws ec2 delete-launch-template --launch-template-name "$NAME" || log "no launch template, or not deleted"
    sg="$(read_aws ec2 describe-security-groups --filters "Name=group-name,Values=$NAME" \
        Name=tag:Project,Values=taxman --query 'SecurityGroups[].GroupId' --output text)"
    if [ -n "$sg" ]; then
        tries=0
        # The network interface of a terminated instance goes a minute or two after it.
        until change_aws ec2 delete-security-group --group-id "$sg"; do
            tries=$((tries + 1))
            [ "$tries" -le 12 ] || die "cannot delete the security group $sg"
            sleep 10
        done
    fi
    change_aws iam delete-role-policy --role-name "$ROLE" --policy-name "$NAME" || log "no policy, or not deleted"

    printf '\nthe end checks (each must print nothing, [] or None):\n'
    read_aws "${live_instances[@]}"
    read_aws ec2 describe-volumes --filters Name=tag:Project,Values=taxman --query 'Volumes[].VolumeId'
    read_aws ec2 describe-spot-instance-requests --filters Name=tag:Project,Values=taxman \
        Name=state,Values=open,active --query 'SpotInstanceRequests[].SpotInstanceRequestId'
    read_aws autoscaling describe-auto-scaling-groups --auto-scaling-group-names "$NAME" \
        --query 'AutoScalingGroups[].AutoScalingGroupName'
    read_aws ec2 describe-launch-templates --filters "Name=launch-template-name,Values=$NAME" \
        --query 'LaunchTemplates[].LaunchTemplateName'
    read_aws ec2 describe-security-groups --filters Name=tag:Project,Values=taxman --query 'SecurityGroups[].GroupId'
    read_aws iam list-role-policies --role-name "$ROLE" --query 'PolicyNames'
    printf '\nthe state of the run stays in %s (results, checkpoints, logs); the bucket and the role %s are kept.\n' \
        "$S3_DEST" "$ROLE"
}

# --- main --------------------------------------------------------------------------------------------------------

command="${1:-}"
[ "$#" -gt 0 ] && shift
case "$command" in
    launch) launch "$@" ;;
    status) status ;;
    stop) stop "$@" ;;
    cleanup) cleanup "$@" ;;
    -h | --help | help) usage ;;
    *) die "usage: wall.sh launch --commit SHA --until DATE [options] | status | stop [--now] | cleanup [--force]" ;;
esac
