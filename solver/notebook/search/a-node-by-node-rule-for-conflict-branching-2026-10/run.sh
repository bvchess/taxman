#!/bin/bash
# usage: run.sh SETFILE IDSFILE OUTDIR NODELIMIT
# Each line of SETFILE is "NAME BINARY ENV ARGS..." (ENV a comma-separated list of VAR=VALUE for the environment, or -;
# ARGS tune solve arguments, where $STORE_ARGS stands for the game's carried store, if it has one).  Each line of
# IDSFILE is a unit id (GAME-INDEX).  The settings run one after another on each unit (interleaved per unit), P units at
# once (default 8) at nice 19; resumable: a finished NAME-ID.json is skipped.
export SET=$1 OUT=$3 NL=$4
export K=/home/brian/taxman_work/chain-data/results/strict.jsonl
mkdir -p $OUT
one() {
  id=$1
  game=${id%%-*}
  case $game in
    4983) units=/home/brian/taxman_work/lab/work/hardgames/forecast/units/live4983.jsonl; store= ;;
    4984) units=/home/brian/taxman_work/lab/work/patch/units-4984.jsonl
          store=/home/brian/taxman_work/lab/work/carry/s1/store-4983-144.txt ;;
    4991) units=/home/brian/taxman_work/lab/work/patch4991/units-4991.jsonl
          store=/home/brian/taxman_work/chain-data/carried/nogoods-4984.txt ;;
    4992) units=/home/brian/taxman_work/lab/work/patch4992/units-4992.jsonl
          store=/home/brian/taxman_work/chain-data/carried/nogoods-4991.txt ;;
    4995) units=/home/brian/taxman_work/lab/work/patch4995/units-4995.jsonl
          store=/home/brian/taxman_work/chain-data/carried/nogoods-4992.txt ;;
    *) echo "unknown game $game" >&2; return ;;
  esac
  if [ -n "$store" ]; then STORE_ARGS="--tactic carried_nogoods=on --carried $store"; else STORE_ARGS=; fi
  while read s bin envs args; do
    [ -z "$s" ] && continue
    [ -s $OUT/$s-$id.json ] && continue
    [ "$envs" = "-" ] && envs=
    eval "env ${envs//,/ } nice -n 19 $bin solve $game --known $K --units $units --ids $id $args --node-limit $NL" \
      > $OUT/$s-$id.tmp 2> $OUT/$s-$id.err && mv $OUT/$s-$id.tmp $OUT/$s-$id.json
  done < $SET
}
export -f one
grep -v '^#' $2 | awk '{print $1}' | xargs -P ${P:-8} -I{} bash -c 'one {}'
touch $OUT/DONE
