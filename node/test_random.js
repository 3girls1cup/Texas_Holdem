import { SecretNetworkClient, Wallet, MsgExecuteContract } from "secretjs";
import * as fs from "fs";
// import dotenv from "dotenv";
// dotenv.config();

// const wallet = new Wallet(process.env.MNEMONIC);
const wallet = new Wallet("pigeon desk hammer sleep only mistake stool december offer patrol once vacant");
const wallet2 = new Wallet("desk pigeon hammer sleep only mistake stool december offer patrol once vacant");
const wallet3 = new Wallet("hammer desk pigeon sleep only mistake stool december offer patrol once vacant");
const createSecretNetworkClient = (wallet) => {
  return new SecretNetworkClient({
    chainId: "pulsar-3",
    url: "https://pulsar.lcd.secretnodes.com",
    wallet: wallet,
    walletAddress: wallet.address,
  });
};

const client1 = createSecretNetworkClient(wallet);
const client2 = createSecretNetworkClient(wallet2);
const client3 = createSecretNetworkClient(wallet3);

let contractInfo = {
  contractAddress: "",
  contractCodeHash: "",
}

let contractInfo2 = {
  contractAddress: "",
  contractCodeHash: "",
}

const loadContractInfo = (contractInfo, contractInfoPath) => {
  if (fs.existsSync(contractInfoPath)) {
    const contractInfoData = fs.readFileSync(contractInfoPath, "utf8");
    contractInfo = JSON.parse(contractInfoData);
    console.log("Contract info loaded:", contractInfo);
  } else {
    console.error("Contract info file not found:", contractInfoPath);
  }
  return contractInfo;
};

const contractInfoPath = "contractInfo.json";
const contractInfo2Path = "contractInfo2.json";
contractInfo = loadContractInfo(contractInfo, contractInfoPath);
contractInfo2 = loadContractInfo(contractInfo2, contractInfo2Path);

const msg = {
  showdown: {
    table_id: 42,
    show_cards: [
      wallet.address,
      wallet2.address,
      wallet3.address,
    ],
    all_in_showdown: true,
  }
};

let execute = async (address, secretjs, info) => {
  try {
    const flip_tx = await secretjs.tx.compute.executeContract(
      {
        sender: address,
        contract_address: info.contractAddress,
        msg: msg,
        code_hash: info.contractCodeHash,
      },
      { gasLimit: 40_000 }
    );
    console.log(flip_tx);
  } catch (error) {
    console.error("Error executing contract:", error);
  }
};

execute(wallet.address, client1, contractInfo);
// execute(wallet.address, client1, contractInfo);

let trx = (address, info, msg) => {
  let trx = new MsgExecuteContract({
    sender: address,
    contract_address: info.contractAddress,
    msg: msg,
    code_hash: info.contractCodeHash,
  });

  return trx;
};

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
      [trx(address, info, msg)],
      { gasLimit: 32_581,
        broadcastMode: "Sync" 
      }
    );

    console.log(flip_tx);
  } catch (error) {
    console.error("Error broadcasting transaction:", error);
  }
};

const random_request = {
  random_request: {}
};
// broadcast(wallet2.address, createSecretNetworkClient(wallet2), contractInfo, random_request);