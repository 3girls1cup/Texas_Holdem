import { SecretNetworkClient, Wallet } from "secretjs";
import * as fs from "fs";
import { client1} from "./shared.js";
// import dotenv from "dotenv";
// dotenv.config();

// const wallet = new Wallet(process.env.MNEMONIC);
const wallet = new Wallet("pigeon desk hammer sleep only mistake stool december offer patrol once vacant");
const wallet2 = new Wallet("desk pigeon hammer sleep only mistake stool december offer patrol once vacant");
const wallet3 = new Wallet("hammer desk pigeon sleep only mistake stool december offer patrol once vacant");
const wallet4 = new Wallet("pigeon desk hammer sleep mistake only stool december offer patrol once vacant");


let contractInfo = {
  contractAddress: "",
  contractCodeHash: "",
}
const s: import("secretjs/dist/extensions/snip1155/types/send.js").SendAction
const contractInfoPath = "contractInfo.json";
if (fs.existsSync(contractInfoPath)) {
  const contractInfoData = fs.readFileSync(contractInfoPath, "utf8");
  contractInfo = JSON.parse(contractInfoData);
  console.log("Contract info loaded:", contractInfo);
} else {
  console.error("Contract info file not found:", contractInfoPath);
}

let permitName = "query_cards";
let allowedTokens = [contractInfo.contractAddress];
let chainId = "pulsar-3";
let permissions = ["allowance"];



let getSignature =  async (wallet) => {
    const { signature } = await wallet.signAmino(
        wallet.address,
        {
        chain_id: chainId,
        account_number: "0", // Must be 0
        sequence: "0", // Must be 0
        fee: {
            amount: [{ denom: "uscrt", amount: "0" }], // Must be 0 uscrt
            gas: "1", // Must be 1
        },
        msgs: [
            {
            type: "query_permit", // Must be "query_permit"
            value: {
                permit_name: permitName,
                allowed_tokens: allowedTokens,
                permissions: permissions,
            },
            },
        ],
        memo: "", // Must be empty
        },
        {
        preferNoSetFee: true, // Fee must be 0, so hide it from the user
        preferNoSetMemo: true, // Memo must be empty, so hide it from the user
        }
    );
    return signature;
    };

let query_cards = async (secretjs, signature) => {
  const res = await secretjs.query.compute.queryContract(
    {
      contract_address: contractInfo.contractAddress,
      code_hash: contractInfo.contractCodeHash,
      query: {
        with_permit: {
          query: { get_player_cards: {table_id: 1} },
          permit: {
            params: {
              permit_name: permitName,
              allowed_tokens: allowedTokens,
              chain_id: chainId,
              permissions: permissions,
            },
            signature: signature,
          },
        },
      },
    },
  );

  console.log(res);
};
let signature = await getSignature(wallet);
// console.log(wallet2.address);
try {
  query_cards(client1, signature);
} catch (error) {
  console.error("Error querying contract:", error);
}
