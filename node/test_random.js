import { wallet, loadContractInfo, contractInfo, trx, start_game, flop, showdown_all_in, client1, execute, turn, river, showdown} from "./shared.js";

const contractInfo2 = loadContractInfo("contractInfo2.json");

let measureTrxTime = (address, info) => {
  console.time("trxTime");
  for (let i = 0; i < 10000; i++) {
    trx(address, info);
  }
  console.timeEnd("trxTime");
};

// measureTrxTime(wallet.address, contractInfo);

let broadcast = async (address, secretjs, info, msg) => {
  try {
    const flip_tx = await secretjs.tx.broadcast(
      [trx(address, info, msg), trx(address, info, flop), trx(address, info, showdown_all_in)],
      { gasLimit: 120_000,
        broadcastMode: "Sync" 
      }
    );

    console.log(flip_tx);
  } catch (error) {
    console.error("Error broadcasting transaction:", error);
  }
};

// broadcast(wallet.address, client1, contractInfo, start_game );

execute(wallet.address, client1, contractInfo, showdown);