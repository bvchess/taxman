#!/bin/bash
# panel.sh BIN LABEL P [tune args...]: the 48 units of the 4895 study, P at a time, 150 CPU s each, env passed through
cd /home/brian/taxman-lab/work/research-cb
bin=$1; label=$2; P=$3; shift 3
tr , "\n" < panel-ids.txt | xargs -P $P -I{} ./unit.sh $bin $label {} --cpu-limit 150 "$@"
echo "panel $label done"
