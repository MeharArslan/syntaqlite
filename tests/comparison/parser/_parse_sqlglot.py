import sqlglot,sys
list(sqlglot.parse(open(sys.argv[1]).read(),dialect='sqlite'))
