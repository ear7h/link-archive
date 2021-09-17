#!/bin/bash

cat $(find migrations -type file | sort)
