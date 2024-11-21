#!/bin/bash

if [[ $1 == *"/deps/"* ]]; then
  _OP=_test
else
  _OP=_run
fi

just $_OP "$1"
