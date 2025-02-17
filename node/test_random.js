import {wallet2, wallet3,createSecretNetworkClient, wallet, loadContractInfo, contractInfo, trx, start_game, flop, showdown_all_in, client1, execute, turn, river, showdown} from "./shared.js";
import * as fs from "fs";
const contractInfo2 = loadContractInfo("contractInfo2.json");


// measureTrxTime(wallet.address, contractInfo);
// console.log(wallet3.address);

let broadcast = async (address, secretjs, info, msg) => {
  try {
    const response = await secretjs.tx.broadcast(
      [trx(address, info, msg)],
      { gasLimit: 40_000,
        broadcastMode: "Sync" 
      }
    );
    fs.writeFileSync("response.json", JSON.stringify(response, null, 2));
    console.log(response);
  } catch (error) {
    console.error("Error broadcasting transaction:", error);
  }
};

let measureTrxTime = async (address, info) => {
  console.time("trxTime");
  for (let i = 0; i < 10; i++) {
  
  }
  await broadcast(wallet.address, client1, contractInfo, start_game);

  console.timeEnd("trxTime");
};

measureTrxTime(wallet.address, contractInfo);


// execute(wallet.address, client1, contractInfo, start_game);