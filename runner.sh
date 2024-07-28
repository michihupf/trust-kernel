#!/bin/bash

if [[ $1 == *"/deps/"* ]]; then
  _MAKE_OP=test
else
  _MAKE_OP=run
fi


make $_MAKE_OP KERNEL_BIN="$1"
