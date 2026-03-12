#!/bin/bash
npx prettier --stdin-filepath test.sql < "$1"
