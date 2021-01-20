#!/bin/bash

sudo yum -y update
sudo amazon-linux-extras enable java-openjdk11
sudo yum install -y java-11-openjdk
sudo yum clean metadata
