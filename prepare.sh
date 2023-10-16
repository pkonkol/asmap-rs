#!/usr/bin/bash
set -e

if [ ! -d "inputs" ]; then
    mkdir inputs
fi
#python -m venv inputs/venv
#source inputs/venv/bin/activate
#pip install graphqlclient
whois --help >/dev/null 2>&1 # check whether whois is installed
# TODO move this to rust code
#if [ ! -f "asrank-download.py" ]; then
#    wget https://api.asrank.caida.org/dev/scripts/asrank-download.py -O inputs/asrank-download.py
#fi
#if [ ! -f "inputs/asns.jsonl" ]; then
#    python3 inputs/asrank-download.py -v -a inputs/asns.jsonl -u https://api.asrank.caida.org/v2/graphql
#fi
#echo "all required files are in place"
echo "requirements in place"
