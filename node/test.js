import * as fs from "fs";

let jsonRes;

const path = "flip_tx.json";
if (fs.existsSync(path)) {
  const data = fs.readFileSync(path, "utf8");
  jsonRes = JSON.parse(data);
} else {
  console.error("Contract info file not found:", path);
}



let parsedResults = [];

for (let log of jsonRes.jsonLog) {
  if (log.events) {
    for (let event of log.events) {
      if (event.type === "wasm") 
      {
        for (let attribute of event.attributes) {
            if (attribute.key === "response") {
                const parsedRes = JSON.parse(attribute.value);
                parsedResults.push(parsedRes);
                break;
            }
        }
      }
    }
  }
}

console.log(parsedResults);