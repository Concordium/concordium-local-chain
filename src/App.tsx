import React, { useEffect, useState } from "react";
import { CModalFooter, CModal, CModalHeader, CModalBody, CModalTitle } from "@coreui/react";
import {
     BrowserRouter as Router,
     Route,
     Routes,
     useNavigate,
} from "react-router-dom";
import { invoke } from "@tauri-apps/api/tauri";
import { faCircleInfo } from "@fortawesome/free-solid-svg-icons";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { CAlert, CButton, CFormLabel, CFormSelect } from "@coreui/react";
import { open } from "@tauri-apps/api/shell";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import ExpertSettingsPage from "./components/expert-form";
import AdvancedSettingsPage from "./components/advanced-form";
import SettingsPage from "./components/form";
/* --------------------------------------------------------- INSTALLATION PAGE ----------------------------------------------------------------------*/

function Installer() {
     // Installation for Node
     const [installing, setInstalling] = useState(false);
     // Installation for Genesis Creator
     const [installingCreator, setInstallingCreator] = useState(false);

     const [verifying, setVerifying] = useState(false);
     const [verificationError, setVerificationError] = useState<string | null>(
          null,
     );
     const [verificationSuccess, setVerificationSuccess] = useState(false);

     // Verification for Node
     const [installationSuccess, setInstallationSuccess] = useState(false);
     // Verification for Genesis Creator
     const [installationSuccessCreator, setInstallationSuccessCreator] =
          useState(false);

     const navigate = useNavigate();

     function goToGenesisBuilder() {
          navigate("/genesis-builder");
     }

     async function install() {
          setInstalling(true);
          try {
               await invoke("install");
               setInstallationSuccess(true);
          } catch (error) {
               console.error("Installation error:", error);
          } finally {
               setInstalling(false);
          }
     }

     async function verifyInstallation() {
          setVerifying(true);
          try {
               await invoke("verify_installation");
               setVerificationError(null); // clear any previous errors
               setVerificationSuccess(true);
               setInstallationSuccess(true);
               console.error("Verification Failed: ", error);
               setVerificationError("Verification Failed: " + error);
          } finally {
               setVerifying(false);
          }
     }

     async function installCreator() {
          setInstallingCreator(true);
          try {
               await invoke("install_genesis_creator");
               setInstallationSuccessCreator(true);
          } catch (error) {
               console.error("Installation error:", error);
          } finally {
               setInstallingCreator(false);
          }
     }
     return (
          <div className="container prose md:prose-lg lg:prose-xl m-auto">
               <h1 className=" subpixel-antialiased text-5xl font-bold text-white">
                    Concordium LC1C
               </h1>

               <p className="my-5 text-xl text-slate-300">
                    Follow the below steps to complete installation and running of a local
                    node.
               </p>

               <CButton
                    className={`bg-ctp-blue mx-auto w-4/5 sm:w-34 md:w-44  text-lg inline-flex items-center justify-center  text-center font-extrabold text-normal text-slate-800 hover:text-slate-700 lg:px-8 xl:px-10 mb-3 ${installing
                         ? "bg-ctp-sky"
                         : installationSuccess || verificationSuccess
                              ? "bg-ctp-green"
                              : "bg-ctp-blue hover:bg-ctp-sky"
                         }`}
                    onClick={install}
                    disabled={installing || installationSuccess}
               >
                    {installing
                         ? "Installing..."
                         : installationSuccess || verificationSuccess
                              ? "Node Installed!"
                              : "Install Concordium Node"}
               </CButton>

               <CButton
                    className={`bg-ctp-blue mx-auto w-4/5 sm:w-34 md:w-44  text-lg inline-flex items-center justify-center  text-center font-extrabold text-normal text-slate-800 hover:text-slate-700 lg:px-8 xl:px-10 mb-3 ${verifying
                         ? "bg-ctp-sky"
                         : verificationSuccess
                              ? "bg-ctp-green"
                              : "bg-ctp-blue hover:bg-ctp-sky"
                         }`}
                    onClick={verifyInstallation}
                    disabled={verifying || verificationSuccess}
               >
                    {verifying
                         ? "Verifying Installation..."
                         : verificationSuccess
                              ? "Node Installation Verified!"
                              : "Verify Node Installation"}
               </CButton>

               <CButton
                    className={`bg-ctp-blue mx-auto w-4/5 sm:w-34 md:w-44 hover:bg-ctp-sky text-lg inline-flex items-center justify-center  text-center font-extrabold text-normal text-slate-800 hover:text-slate-700 lg:px-8 xl:px-10 mb-3 ${installingCreator
                         ? "bg-ctp-sky"
                         : installationSuccessCreator
                              ? "bg-ctp-green"
                              : "bg-ctp-blue hover:bg-ctp-sky"
                         }`}
                    onClick={installCreator}
                    disabled={installingCreator || installationSuccessCreator}
               >
                    {installingCreator
                         ? "Installing..."
                         : installationSuccessCreator
                              ? "Genesis Creator Installed!"
                              : "Install Genesis Creator"}
               </CButton>

               {installationSuccess && installationSuccessCreator && (
                    <CButton
                         className="bg-ctp-blue hover:bg-ctp-sky mx-auto w-4/5 sm:w-34 md:w-44  text-lg inline-flex items-center justify-center  text-center font-extrabold text-normal text-slate-800 hover:text-slate-700 lg:px-8 xl:px-10 mb-3"
                         onClick={goToGenesisBuilder}
                    >
                         Go to Genesis Builder
                    </CButton>
               )}

               {verificationError && <div className="error">{verificationError}</div>}
          </div>
     );
}

/* --------------------------------------------------------- GENESIS BUILDER PAGE ----------------------------------------------------------------------*/


function GenesisBuilder() {
     const [configLevel, setConfigLevel] = useState<string | null>(null);
     const [launching, setLaunching] = useState(false);
     const [launched, setLaunched] = useState(false);
     const [tomlData, settomlData] = useState(null);
     const [chainFolders, setChainFolders] = useState<string[]>([]);
     const [selectedFolder, setSelectedFolder] = useState<string | null>(null);

     const [formData, setFormData] = useState({
          protocolVersion: "5",
          out: {
               updateKeys: "./update-keys",
               accountKeys: "./accounts",
               bakerKeys: "./bakers",
               identityProviders: "./idps",
               anonymityRevokers: "./ars",
               genesis: "./genesis.dat",
               cryptographicParameters: "./global",
               deleteExisting: true,
               genesisHash: "./genesis_hash"
          },
          cryptographicParameters: {
               kind: "generate",
               genesisString: "Local genesis parameters."
          },
          anonymityRevokers: [
               {
                    kind: "fresh",
                    id: 1,
                    repeat: 3
               }
          ],
          identityProviders: [
               {
                    kind: "fresh",
                    id: 0,
                    repeat: 3
               }
          ],
          accounts: [
               {
                    kind: "fresh",
                    balance: "3500000000000000",
                    stake: "3000000000000000",
                    template: "baker",
                    identityProvider: 0,
                    numKeys: 1,
                    threshold: 1,
                    repeat: 1
               },
               {
                    kind: "fresh",
                    balance: "10000000000000000",
                    template: "foundation",
                    identityProvider: 0,
                    numKeys: 1,
                    threshold: 1,
                    repeat: 1,
                    foundation: true
               },
               {
                    kind: "fresh",
                    balance: "2000000000000",
                    template: "stagenet",
                    identityProvider: 0,
                    numKeys: 1,
                    threshold: 1,
                    repeat: 100
               }
          ],
          updates: {
               root: {
                    threshold: 5,
                    keys: [
                         {
                              kind: "fresh",
                              repeat: 7
                         }
                    ]
               },
               level1: {
                    threshold: 7,
                    keys: [
                         {
                              kind: "fresh",
                              repeat: 15
                         }
                    ]
               },
               level2: {
                    keys: [
                         {
                              kind: "fresh",
                              repeat: 7
                         }
                    ],
                    emergency: { 
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    protocol: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    electionDifficulty: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    euroPerEnergy: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    microCCDPerEuro: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    foundationAccount: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    mintDistribution: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    transactionFeeDistribution: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    gasRewards: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    poolParameters: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    addAnonymityRevoker: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    addIdentityProvider: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    cooldownParameters: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    },
                    timeParameters: {
                         authorizedKeys: [
                              0,
                              1,
                              2,
                              3,
                              4,
                              5,
                              6
                         ],
                         threshold: 7
                    }
               }
          },
          parameters: {
               slotDuration: 250,
               leadershipElectionNonce: "d1bc8d3ba4afc7e109612cb73acbdddac052c93025aa1f82942edabb7deb82a1",
               epochLength: 900,
               maxBlockEnergy: 3000000,
               finalization: {
                    minimumSkip: 0,
                    committeeMaxSize: 1000,
                    waitingTime: 100,
                    skipShrinkFactor: 0.5,
                    skipGrowFactor: 2,
                    delayShrinkFactor: 0.5,
                    delayGrowFactor: 2,
                    allowZeroDelay: true
               },
               chain: {
                    version: "v1",
                    electionDifficulty: 0.05,
                    euroPerEnergy: 0.000001,
                    microCCDPerEuro: 100000000,
                    accountCreationLimit: 10,
                    timeParameters: {
                         rewardPeriodLength: 4,
                         mintPerPayday: 0.000261157877
                    },
                    poolParameters: {
                         passiveFinalizationCommission: 1,
                         passiveBakingCommission: 0.1,
                         passiveTransactionCommission: 0.1,
                         finalizationCommissionRange: {
                              min: 0.5,
                              max: 1
                         },
                         bakingCommissionRange: {
                              min: 0.05,
                              max: 0.1
                         },
                         transactionCommissionRange: {
                              min: 0.05,
                              max: 0.2
                         },
                         minimumEquityCapital: "100",
                         capitalBound: 0.25,
                         leverageBound: {
                              numerator: 3,
                              denominator: 1
                         }
                    },
                    cooldownParameters: {
                         poolOwnerCooldown: 3600,
                         delegatorCooldown: 1800
                    },
                    rewardParameters: {
                         mintDistribution: {
                              bakingReward: 0.6,
                              finalizationReward: 0.3
                         },
                         transactionFeeDistribution: {
                              baker: 0.45,
                              gasAccount: 0.45
                         },
                         gASRewards: {
                              baker: 0.25,
                              finalizationProof: 0.005,
                              accountCreation: 0.02,
                              chainUpdate: 0.005
                         }
                    }
               }
          }
     });

     const navigate = useNavigate();
     const HandleSubmit = (formData: any) => {
          console.log(formData)
          setFormData(formData)
     }
     const expertHandleSubmit = (tomlData: any) => {
          console.log(tomlData)
          settomlData(tomlData)
     }

     function dashboard() {
          navigate("/dashboard");
     }

     function handleOpenLink(event: React.MouseEvent<HTMLAnchorElement>) {
          event.preventDefault(); // Prevent the default behavior of the link
          open(
               "https://raw.githubusercontent.com/Concordium/concordium-misc-tools/9d347761aadd432cbb6211a7d7ba38cdc07f1d11/genesis-creator/examples/single-baker-example-p5.toml",
          ); // Replace with the link you want to open
     }

     const EasyConfig = () => (
          <div className="text-white">
               <p>
                    For easy configuration, the local chain will be loaded with a template
                    genesis file found{" "}
                    <a
                         href="https://raw.githubusercontent.com/Concordium/concordium-misc-tools/9d347761aadd432cbb6211a7d7ba38cdc07f1d11/genesis-creator/examples/single-baker-example-p5.toml"
                         onClick={handleOpenLink}
                    >
                         here
                    </a>
                    .{" "}
               </p>
               <p> Nothing else is needed. You can successfully run the local chain. </p>
          </div>
     );

     const AdvancedConfig = () => (
          <div className="text-white text-normal text-center">
               <p>
                    This is config settings for Advanced users.
               </p>
               {(launching || launched) ? "" : <SettingsPage formData={formData} setFormData={setFormData} onHandleSubmit={HandleSubmit} />}
          </div>

     );

     const ExpertConfig = () => (
          <div className="text-white text-normal text-center">
               <p>
                    This is config settings for Expert users.
               </p>
               {(launching || launched) ? "" : <ExpertSettingsPage onHandleSubmit={expertHandleSubmit} />}
          </div>
     );

     const FromExisting = () => (
          <div className="mt-4 ">
               <CFormLabel htmlFor="select-option" className="text-white text-xl mb-3 ml-1">Select a chain folder:</CFormLabel>
               <CFormSelect
                    id="select-option"
                    className="ml-2 px-4 py-2 bg-white text-black w-full"
                    value={selectedFolder || ""}
                    onChange={(e) => setSelectedFolder(e.target.value)}
               >
                    <option value="" disabled>
                         Select folder
                    </option>
                    {chainFolders.map((folder) => (
                         <option key={folder} value={folder}>
                              {folder}
                         </option>
                    ))}
               </CFormSelect>
          </div>
     )

     async function launch() {
          setLaunching(true);
          let launch_mode = null;
          console.log(configLevel)

          if (configLevel === "easy") {
               launch_mode = { Easy: null };
          } else if (configLevel === "advanced") {
               launch_mode = { Advanced: JSON.stringify(formData) };
          } else if (configLevel === "existing") {
               launch_mode = { FromExisting: selectedFolder };
          } else {
               launch_mode = { Expert: tomlData };
          }
          try {
               console.log(launch_mode)
               await invoke('launch_template', { launchMode: launch_mode });
               setLaunching(false);
               setLaunched(true);
          } catch (error) {
               console.error("Error calling launch_template:", error);
               setLaunching(false);
          }
     }
     useEffect(() => {
          async function fetchChainFolders() {
               try {
                    localStorage.clear();
                    const folders = await invoke('list_chain_folders');
                    setChainFolders(folders as any);
               } catch (error) {
                    console.error("Error fetching chain folders:", error);
               }
          }
          fetchChainFolders();
     }, []);

     return (
          <>
               <CAlert color="primary" className="m-10 text-lg text-ctp-black">
                    <FontAwesomeIcon icon={faCircleInfo} className="mr-2" />
                    This determines the complexity of the configuration options available to
                    you.
               </CAlert>
               <div className="flex flex-col items-center justify-center text-white">
                    <h2 className="text-2xl font-semibold mb-4 text-center">
                         Choose Level of Expertise:
                    </h2>
                    <div className="space-x-4 mb-8 md:w-full w-1/2 flex justify-center items-center gap-1 m-10">
                         <CButton
                              className="flex-grow w-40 px-4 py-2 font-semibold text-white bg-blue-500 rounded-lg disabled:opacity-50 transition-colors duration-300 focus:bg-blue-600 active:bg-blue-700"
                              onClick={() => setConfigLevel("easy")}
                              disabled={launched}
                         >
                              Easy
                         </CButton>

                         <CButton
                              className="flex-grow w-40 px-4 py-2 font-semibold text-white bg-blue-500 rounded-lg disabled:opacity-50 transition-colors duration-300 focus:bg-blue-600 active:bg-blue-700"
                              onClick={() => setConfigLevel("advanced")}
                              disabled={launched}
                         >
                              Advanced
                         </CButton>
                         <CButton
                              className="flex-grow w-40 px-4 py-2 font-semibold text-white bg-blue-500 rounded-lg disabled:opacity-50 transition-colors duration-300 focus:bg-blue-600 active:bg-blue-700"
                              onClick={() => setConfigLevel("expert")}
                              disabled={launched}
                         >
                              Expert
                         </CButton>
                         <CButton
                              className="flex-grow w-40 px-4 py-2 font-semibold text-white bg-blue-500 rounded-lg disabled:opacity-50 transition-colors duration-300 focus:bg-blue-600 active:bg-blue-700"
                              onClick={() => setConfigLevel("existing")}
                              disabled={launched}
                         >
                              Load Config
                         </CButton>

                    </div>
                    <div className="config-container w-4/5">
                         {configLevel === "easy" && <EasyConfig />}
                         {configLevel === "advanced" && <AdvancedConfig />}
                         {configLevel === "expert" && <ExpertConfig />}
                         {configLevel === "existing" && <FromExisting />}
                    </div>

                    <div className=" mt-8">
                         <button
                              className="px-4 py-2 font-semibold text-white bg-green-500 rounded-lg hover:bg-green-600 focus:outline-none focus:ring-2 focus:ring-green-400"
                              onClick={launch}
                              disabled={launching || launched}
                              style={{
                                   backgroundColor: launched ? "green" : undefined,
                                   color: launched ? "white" : undefined,
                              }}
                         >
                              {launching
                                   ? "Launching..."
                                   : launched
                                        ? "Chain Launched!"
                                        : "Launch Local Chain"}
                         </button>
                         {launched ? (
                              <button
                                   className="ml-4 px-4 py-2 font-semibold text-white bg-blue-500 rounded-lg hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-400"
                                   onClick={dashboard}
                              >
                                   Visit Dashboard
                              </button>
                         ) : (
                              ""
                         )}
                    </div>
               </div>
          </>
     );
}

/* --------------------------------------------------------- DASHBOARD PAGE ----------------------------------------------------------------------------*/

function Dashboard() {
     const [latestHash, setLatestHash] = useState("");
     const [blocks, setBlocks] = useState("");
     const [amountDict, setAmounts] = useState({});
     const [amountDictFilter, setAmountsFilter] = useState({});
     const [filterValue, setFilter] = useState('');
     const [tempDict, setTempDict] = useState({});
     useEffect(() => {
          let unlistenFn: UnlistenFn | undefined;

          // Set up the listener for the 'new-block' event
          listen("new-block", (event: any) => {
               setBlocks(event.payload.number);
               setLatestHash(event.payload.hash);
               setAmounts(event.payload.amounts);
               if (filterValue.length == 0) {
                    setTempDict(event.payload.amounts);
               }
          })
               .then((unlisten) => {
                    unlistenFn = unlisten;
               })
               .catch((error) => {
                    console.error("Error setting up listener:", error);
               });

          // Cleanup the listener when the component is unmounted
          return () => {
               if (unlistenFn) {
                    unlistenFn();
               }
          };
     }, []);

     const killPopup = () => {
          const [visible, setVisible] = useState(false)
          const openModalAndKillChain = async () => {
               setVisible(true);
               localStorage.clear();
               await killChain();
          }
          return (
               <>
                    <CButton className="bg-red-800 border-red-800 hover:bg-red-800 hover:border-red-800 w-40" onClick={openModalAndKillChain}>Kill Chain</CButton>
                    <CModal
                         className="rounded-2xl"
                         alignment="center"
                         backdrop="static"
                         visible={visible}
                    >
                         <CModalHeader closeButton={false} className="bg-slate-800 text-white font-bold  border-black">
                              <CModalTitle className="text-xl">Note:</CModalTitle>
                         </CModalHeader>
                         <CModalBody className="bg-slate-800 text-white">
                              <p>Chain has been killed.</p>
                         </CModalBody>
                         <CModalFooter className="bg-slate-800 border-black">
                              <CButton color="primary" className="rounded-full border-rose-500 bg-rose-500 hover:bg-rose-800 hover:border-rose-800 " onClick={() => window.location.href = "/genesis-builder"}>Go to Genesis Builder</CButton>
                         </CModalFooter>
                    </CModal>
               </>
          )
     }


     // Function to invoke killing blockchain
     async function killChain() {
          try {
               await invoke("kill_chain");
               console.log("killed chain")
               // window.location.href = "/genesis-builder";
          } catch (error) {
               console.error("Kill error:", error);
          }
     }

     function filter(e: any) {
          setFilter(e.target.value);
          localStorage.setItem('dictionary', JSON.stringify(amountDict));
          var dictionary = JSON.parse(localStorage.getItem('dictionary') as string);

          Object.keys(dictionary).map(x => {
               if (x.indexOf(e.target.value) == -1) {
                    delete dictionary[x];
               }
          })
          setAmountsFilter(dictionary);
     }

     return (
          <div className="">
               <div className="">
                    <div className="" style={{ position: 'absolute', top: '3vh', left: 0, color: 'whitesmoke', fontSize: 'larger', padding: '0 2.5vw 0 2.5vw' }}>
                         <div className="heading" style={{ display: 'flex', justifyContent: 'space-between', width: '95vw', alignItems: 'center', marginBottom: '2vh' }}>
                              <div className="" style={{ fontSize: 'larger' }}>
                                   Information
                              </div>
                              {killPopup()}
                         </div>
                         <div className="main" style={{ color: 'whitesmoke', fontSize: 'larger', display: 'flex', justifyContent: 'space-between', width: '95vw' }}>
                              <div className="" style={{ border: '1px solid #12172b', borderRadius: '10px', backgroundColor: '#1c2445', padding: '20px', width: '46.3vw' }}>
                                   <div className="right" style={{ color: '#de14d9' }}>
                                        <p className="" style={{ fontWeight: '500', fontSize: '20px', marginBottom: '10px' }}>
                                             BLOCK NUMBER
                                        </p>
                                        <p className="" style={{ fontSize: '20px' }}>
                                             {blocks}
                                        </p>
                                   </div>
                              </div>
                              <div className="" style={{ border: '1px solid #12172b', borderRadius: '10px', backgroundColor: '#1c2445', padding: '20px', width: '46.3vw' }}>
                                   <div className="right " style={{ color: '#09e030' }}>
                                        <p className="" style={{ fontWeight: '500', fontSize: '20px', marginBottom: '10px' }}>
                                             LATEST HASH
                                        </p>
                                        <p className="overflow-x-scroll scrollbar-thin" style={{ fontSize: '20px' }}>
                                             {latestHash}
                                        </p>
                                   </div>
                              </div>
                         </div>
                         <br />
                         <div className="search" id="search" style={{ width: '95vw', display: 'flex' }}>
                              <svg xmlns="http://www.w3.org/2000/svg" x="0px" y="0px" width="50" height="50" viewBox="0 0 24 24" style={{ flex: 'none', width: '5vw' }}>
                                   <path d="M22 20L20 22 14 16 14 14 16 14z" style={{ fill: 'whitesmoke' }}></path>
                                   <path d="M9,16c-3.9,0-7-3.1-7-7c0-3.9,3.1-7,7-7c3.9,0,7,3.1,7,7C16,12.9,12.9,16,9,16z M9,4C6.2,4,4,6.2,4,9c0,2.8,2.2,5,5,5 c2.8,0,5-2.2,5-5C14,6.2,11.8,4,9,4z" style={{ fill: 'whitesmoke' }}>
                                   </path>
                                   <path d="M13.7 12.5H14.7V16H13.7z" transform="rotate(-44.992 14.25 14.25)" style={{ fill: 'whitesmoke' }}>
                                   </path>
                              </svg>
                              <input type="text" name="search" id="search" style={{ width: '90vw' }} onChange={filter} />
                         </div>
                         <div style={{ height: '70vh', overflow: 'hidden', overflowY: 'scroll', marginTop: '3vh' }}>
                              <div className="table" style={{ borderRadius: '10px!important' }}>
                                   <table style={{ textAlign: "left", width: '95vw', backgroundColor: '#1c2445!important', borderRadius: '10px', border: '1px solid #1c2445', color: 'white!important' }}>
                                        <tr>
                                             <th style={{ backgroundColor: '#1c244550', color: 'white', width: "50vw" }}>Account Address</th>
                                             <th style={{ backgroundColor: '#1c244550', color: 'white' }}>Amount</th>
                                        </tr>
                                        {filterValue.length == 0 && Object.keys(amountDict).map(x => {
                                             return (<><tr key={x}>
                                                  <td className="overflow-x-scroll scollbar-thin" style={{ backgroundColor: '#1c244550', color: 'white', fontWeight: '200' }}>{x}</td>
                                                  <td className="overflow-x-scroll scollbar-thin" style={{ backgroundColor: '#1c244550', color: 'white', fontWeight: '200' }}>{amountDict[x as any]}</td>
                                             </tr></>)
                                        })}
                                        {filterValue.length ? Object.keys(amountDictFilter).map(x => {
                                             return (<><tr key={x}>
                                                  <td className="overflow-x-scroll scollbar-thin" style={{ backgroundColor: '#1c244550', color: 'white', fontWeight: '200' }}>{x}</td>
                                                  <td className="overflow-x-scroll scollbar-thin" style={{ backgroundColor: '#1c244550', color: 'white', fontWeight: '200' }}>{amountDictFilter[x as any]}</td>
                                             </tr></>)
                                        }) : <></>}
                                   </table>
                              </div>
                              {Object.keys(amountDict).length == 0 && <div className="loader" style={{ width: '95vw', transform: 'scale(0.2)', position: 'absolute', top: '-20%', left: '7%' }}>
                                   <svg version="1.1" id="L4" xmlns="http://www.w3.org/2000/svg" xmlns: xlink="http://www.w3.org/1999/xlink" x="0px" y="0px"
                                        viewBox="0 0 100 100" enable-background="new 0 0 0 0" xml: space="preserve">
                                        <circle fill="#ffffff10" stroke="none" cx="10" cy="10" r="6">
                                             <animate
                                                  attributeName="opacity"
                                                  dur="1s"
                                                  values="0;1;0"
                                                  repeatCount="indefinite"
                                                  begin="0.1" />
                                        </circle>
                                        <circle fill="#ffffff10" stroke="none" cx="25" cy="10" r="6">
                                             <animate
                                                  attributeName="opacity"
                                                  dur="1s"
                                                  values="0;1;0"
                                                  repeatCount="indefinite"
                                                  begin="0.2" />
                                        </circle>
                                        <circle fill="#ffffff10" stroke="none" cx="40" cy="10" r="6">
                                             <animate
                                                  attributeName="opacity"
                                                  dur="1s"
                                                  values="0;1;0"
                                                  repeatCount="indefinite"
                                                  begin="0.3" />
                                        </circle>
                                   </svg>
                              </div>}
                         </div>

                    </div>
               </div>
          </div>
     );
}

/* --------------------------------------------------------- FULL APPLICATION ----------------------------------------------------------------------*/

function App() {
     return (
          <Router>
               <Routes>
                    <Route path="/genesis-builder" element={<GenesisBuilder />} />
                    <Route path="/" element={<Installer />} />
                    <Route path="/dashboard" element={<Dashboard />} />
               </Routes>
          </Router>
     );
}

export default App;
