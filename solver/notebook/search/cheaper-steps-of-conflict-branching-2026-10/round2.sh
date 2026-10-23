#!/bin/bash
cd /home/brian/taxman-lab/work/research-stepcost
cp target/release/examples/tune bin/tune-study2
cp target-carry/release/examples/tune bin/tune-carry2
./queue.sh jobs-d1.txt 4
./queue.sh jobs-d2.txt 4
./queue.sh jobs-look2.txt 4
./queue.sh jobs-w.txt 4
echo finished > round2.done
./queue.sh jobs-g.txt 4
echo finished > round3.done
