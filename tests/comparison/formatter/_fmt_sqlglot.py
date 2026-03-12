import sqlglot,sys
sql=open(sys.argv[1]).read()
for e in sqlglot.parse(sql,dialect='sqlite'):
  if e is not None: print(e.sql(dialect='sqlite',pretty=True))
