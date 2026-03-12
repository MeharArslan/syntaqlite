const {Parser}=require("node-sql-parser");const fs=require("fs");
const p=new Parser();
p.astify(fs.readFileSync(process.argv[2],"utf8"),{database:"SQLite"});
