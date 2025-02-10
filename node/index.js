import { SecretNetworkClient, Wallet } from "secretjs";
import * as fs from "fs";
// import dotenv from "dotenv";
// dotenv.config();

// const wallet = new Wallet(process.env.MNEMONIC);
const wallet = new Wallet("desk pigeon hammer sleep only mistake stool december offer patrol once vacant");

const secretjs = new SecretNetworkClient({
  chainId: "pulsar-3",
  url: "https://api.pulsar3.scrttestnet.com",
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
      [0, "secret1xyz...", 987654321],
      [1, "secret1abc...", 123456789],
      [3, "secret1def...", 111222333],
      [7, "secret1ghi...", 999888777],
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

  console.log(flip_tx);
};
try_flip();

let query_flip = async () => {
  let flip_tx = await secretjs.query.compute.queryContract({
    contract_address: "secret1dfsvyqhcs9n32q8lywuuvecvshmnatempuu32r",
    code_hash: "d896d5a9921a95c3302863de9c8e82e99fa9531d7a962d228a1a635f26bc0449",
    query: {
      get_flip: {},
    },
  });
  console.log(flip_tx);
};

// query_flip();
