#!/bin/bash
cp "$1" "$1.bak"; sleek "$1" > /dev/null 2>&1; cp "$1.bak" "$1"; rm -f "$1.bak"
