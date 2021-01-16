#!/bin/bash
cat edges.csv | while read line; do echo ${line:0:1}; done;