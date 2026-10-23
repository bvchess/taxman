#!/bin/bash
cd /home/brian/taxman-lab/work/research-stepcost
until [ -s build-all.log ]; do sleep 10; done
cp target-all/release/examples/tune bin/tune-all
./queue.sh jobs-e1.txt 4
./queue.sh jobs-e2.txt 4
echo finished > round4.done
