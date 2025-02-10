import { SecretNetworkClient, Wallet } from "secretjs";
import * as fs from "fs";
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
    community_cards: {
      table_id: 42,
      game_state: "flop", // "flop" corresponds to 1
    },
  };  
  
  let get_flop = async () => {
    const flop_tx = await secretjs.tx.compute.executeContract(
      {
        sender: wallet.address,
        contract_address: contractInfo.contractAddress,
        msg: msg,
        code_hash: contractInfo.contractCodeHash,
      },
      { gasLimit: 100_000 }
    );
  
    console.log(flop_tx);
  };
get_flop();