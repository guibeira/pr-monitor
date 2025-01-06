import { useState, useEffect } from "react";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";




function App() {
  const [prList, setPrList] = useState([]);
  const [errorMessage, setErrorMessage] = useState("");
  const [activeTab, setActiveTab] = useState("active");
  const [hasToken, setHasToken] = useState(false);
  const [prUrl, setPrUrl] = useState("");

  async function addPr() {
    try {
      const prList = await invoke("add_item", { url: prUrl })
      setPrList(prList);
    } catch (error) {
      updateErrorMessage(error);
    }
    finally {
      setPrUrl("");
    }
  }
  const updateErrorMessage = (message) => {
    setErrorMessage(message);
    setTimeout(() => {
      setErrorMessage("");
    }, 5000);
  }
  useEffect(() => {
    async function hasToken() {
      try {
        const res = await invoke("has_token");
        console.log("has token", res);
        setHasToken(res);
      } catch (error) {
        console.error("Error getting token:", error);
      }
    }

    async function getPrList() {
      try {
        const res = await invoke("get_pr_list");
        console.log("get_pr_list", res);
        setPrList(res);
      } catch (error) {
        console.error("Deu erro:", error);
      }
    }
    hasToken();
    getPrList();

  }, []);


  useEffect(() => {
    const unlisten = listen("error-event", (event) => {
      console.log("Emit Event: ", event.payload); // "Hello from backend!"
      setErrorMessage(event.payload);
      setTimeout(() => {
        setErrorMessage("");
      }, 5000);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen("pr-closed", (event) => {
      console.log("Pr-closed: ",event.payload); // "Hello from backend!"
      // parse to int payload
      const prNumber = parseInt(event.payload);
      setPrList(prList => prList.map(pr => {
        if (pr.pr_number === prNumber) {
          pr.state = "closed";
        }
        return pr;
      }));
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);


  async function startTask() {
    console.log("Starting monitor pull requests")
    try {
      const response = await invoke("start_task");
      console.log(response);
    } catch (error) {
      console.error(error);
    }
  }

  useEffect(() => {
    startTask();
    return () => {
      stopTask();
    }
  }, []);

  async function stopTask() {
    console.log("Stopping monitor pull requests")
    try {
      const response = await invoke("stop_task");
      console.log(response);
    } catch (error) {
      console.error(error);
    }
  }

  async function emitEvent() {
    console.log("Emitting event")
    try {
      const response = await invoke("emit_event");
      console.log(response);
    } catch (error) {
      console.error(error);
    }
  }

  const handleSubmitForm = (e) => {
    e.preventDefault();
    console.log("submit form");
    const token = document.getElementById("token-input").value;
    invoke("add_token", { token });
    setHasToken(true);
  }

  const buildUrlFromPr = (pr) => `https://github.com/${pr.owner}/${pr.repo}/pull/${pr.pr_number}`;

  if (!hasToken) {
    return (
      <main className="m-4 text-center bg-gray-200">
        <h1>Enter your github token</h1>
        <form className="mt-2" onSubmit={handleSubmitForm}>
          <input
            className="w-80 h-12 rounded-lg bg-gray-200 placeholder:text-center focus:outline-none"
            id="token-input" placeholder="Enter token" />

          <button className="ml-2 border-2 border-gray-600 rounded-full w-12 h-12 bg-gray-600 text-white" type="submit">➕</button>
        </form>
        <p className="mt-2">
          click <a className="text-gray-500" href="https://github.com/settings/tokens/new" target="_blank" >here</a> to generate your token{" "}
        </p>
      </main >
    )
  }



  const activeTabStyle = "text-gray-600 hover:text-gray-600 dark:text-gray-500 dark:hover:text-gray-500 border-gray-600 dark:border-gray-500"
  const inactiveTabStyle = "dark:border-transparent text-gray-500 hover:text-gray-600 dark:text-gray-400 border-gray-100 hover:border-gray-300 dark:border-gray-700 dark:hover:text-gray-300"

  const prListOpen = prList.filter(pr => pr.state === "open");
  const prListClosed = prList.filter(pr => pr.state === "closed");

  return (
    <main className="container bg-gray-200">
      {errorMessage &&
        <div className="bg-red-100 border border-red-400 w-screen h-14 text-red-700 px-4 py-3 absolute" role="alert">
          <p className="font-bold">Error: {errorMessage}</p>
        </div>
      }
      <div className="border-b border-gray-200 dark:border-gray-700">
        <ul
          className="flex flex-wrap -mb-px text-sm font-medium text-center align-middle"
          id="default-styled-tab"
        >
          <li className="me-2" role="presentation">
            <button
              onClick={() => setActiveTab("active")}
              className={activeTab === "active" ? "inline-block p-4 border-b-2 rounded-t-lg " + activeTabStyle : "inline-block p-4 border-b-2 rounded-t-lg " + inactiveTabStyle}

              id="profile-styled-tab"
              data-tabs-target="#styled-profile"
              type="button"
              role="tab"
              aria-controls="profile"
              aria-selected="false"
            >Open</button>
          </li>
          <li className="me-2" role="presentation">
            <button
              onClick={() => setActiveTab("not-active")}
              className={activeTab === "not-active" ? "inline-block p-4 border-b-2 rounded-t-lg " + activeTabStyle : "inline-block p-4 border-b-2 rounded-t-lg " + inactiveTabStyle}
              id="dashboard-styled-tab"
              data-tabs-target="#styled-dashboard"
              type="button" role="tab" aria-controls="dashboard" aria-selected="false">Closed</button>
          </li>

          <li className="me-2" role="presentation">
            <form
              className="flex justify-between gap-1 mt-2"
              onSubmit={(e) => {
                e.preventDefault();
                addPr();
              }}
            >
              <input
                id="greet-input"
                value={prUrl}
                className="rounded bg-gray-100 focus:outline-none"
                onChange={(e) => setPrUrl(e.currentTarget.value)}
                placeholder="  Enter pr link"
              />
              <button
                className="border-2 border-gray-600 rounded-full w-10 h-10 bg-gray-600 text-white"
                type="submit">➕</button>
            </form>
          </li>
        </ul>
      </div>
      <div
        style={{ display: activeTab === "active" ? "block" : "none" }}
      >
        {prListOpen.length != 0 &&
          <div className="relative overflow-x-auto bg-gray-100">
            <ul className="max-w-md space-y-1 text-gray-500 list-none list-inside overflow-x-hidden dark:text-gray-400">
              {prListOpen.map((pullRequest) => (

                <li key={pullRequest.pr_number} className={`text-nowrap m-2 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-300 w-32 ${pullRequest.title.length > 50 ? "hover:animate-carousel" : ""}`}>
                  <a href={buildUrlFromPr(pullRequest)} target="_blank"
                    className="text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-300">
                    {pullRequest.title}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        }
        {prListOpen.length == 0 && <p className="text-center mt-2 text-gray-600">No open pull requests</p>}
      </div>
      <div style={{ display: activeTab === "not-active" ? "block" : "none" }}>
        {prListClosed.length != 0 &&
          <div className="relative overflow-x-auto bg-gray-100">
            <ul className="max-w-md space-y-1 text-gray-500 list-none list-inside overflow-x-hidden dark:text-gray-400">
              {prListClosed.map((pullRequest) => (
                <li key={pullRequest.pr_number} className={`text-nowrap m-2 text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-300 w-32 ${pullRequest.title.length > 50 ? "hover:animate-carousel" : ""}`}>

                  <a href={buildUrlFromPr(pullRequest)} target="_blank"
                    className="text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-300">
                    {pullRequest.title}</a>
                </li>
              ))}
            </ul>
          </div>
        }
        {prListClosed.length == 0 && <p className="text-center mt-2 text-gray-600">No closed pull requests</p>}
      </div>
      { /* 
        debug buttons
      <button onClick={startTask}>start_task</button>
      <button onClick={stopTask}>stop_task</button> 
      <button onClick={emitEvent}>emit event</button>
      */}
    </main>
  );
}

export default App;
