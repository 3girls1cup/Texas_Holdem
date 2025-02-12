import { SecretNetworkClient, Wallet } from "secretjs";
import * as fs from "fs";
// import dotenv from "dotenv";
// dotenv.config();

const wallet = new Wallet("pigeon desk hammer sleep only mistake stool december offer patrol once vacant");
const wallet2 = new Wallet("desk pigeon hammer sleep only mistake stool december offer patrol once vacant");
const wallet3 = new Wallet("hammer desk pigeon sleep only mistake stool december offer patrol once vacant");

const secretjs = new SecretNetworkClient({
  chainId: "pulsar-3",
  url: "https://pulsar.lcd.secretnodes.com",
  wallet: wallet,
  walletAddress: wallet.address,
});

let contractInfo = {
  contractAddress: "",
  contractCodeHash: "",
}

const contractInfoPath = "contractInfo.json";
if (fs.existsSync(contractInfoPath)) {
  const contractInfoData = fs.readFileSync(contractInfoPath, "utf8");
  contractInfo = JSON.parse(contractInfoData);
  console.log("Contract info loaded:", contractInfo);
} else {
  console.error("Contract info file not found:", contractInfoPath);
}

const msg = {
  start_game: {
    table_id: 42,
    players: [
      wallet.address,
      wallet2.address,
      wallet3.address,
    ],
  },
};

let try_flip = async () => {
  const flip_tx = await secretjs.tx.compute.executeContract(
    {
      sender: wallet.address,
      contract_address: contractInfo.contractAddress,
      msg: msg,
      code_hash: contractInfo.contractCodeHash,
    },
    { gasLimit: 100_000 }
  );

  fs.writeFileSync("flip_tx.json", JSON.stringify(flip_tx, null, 2));
  console.log("Transaction saved to flip_tx.json");
};
try_flip();