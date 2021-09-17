#!/bin/bash

dir=$(git rev-parse --show-toplevel)/sql/migrations
cat $(find $dir -type f | sort)
