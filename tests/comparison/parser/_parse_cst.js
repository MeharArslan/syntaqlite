const {parse}=require("sql-parser-cst");const fs=require("fs");
parse(fs.readFileSync(process.argv[2],"utf8"),{dialect:"sqlite"});
