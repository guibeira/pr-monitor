import { useEffect, useState } from "react";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./style.css";

function App() {
  const [prList, setPrList] = useState([]);
  const [errorMessage, setErrorMessage] = useState("");
  const [activeTab, setActiveTab] = useState("active");
  const [hasToken, setHasToken] = useState(false);
  const [prUrl, setPrUrl] = useState("");
  const [refreshTime, setRefreshTime] = useState(5);
  const [showNotification, setShowNotification] = useState(true);
  const [theme, setTheme] = useState("system");

  useEffect(() => {
    const applyTheme = async () => {
      const savedTheme = await invoke("get_theme").catch(() => "system");
      setTheme(savedTheme);

      document.documentElement.classList.remove("dark", "light");
      if (savedTheme === "dark") {
        document.documentElement.classList.add("dark");
      } else if (savedTheme === "light") {
        document.documentElement.classList.add("light");
      } else {
        if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
          document.documentElement.classList.add("dark");
        } else {
          document.documentElement.classList.add("light");
        }
      }
    };
    applyTheme();

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = (e) => {
      if (theme === "system") {
        document.documentElement.classList.remove("dark", "light");
        if (e.matches) {
          document.documentElement.classList.add("dark");
        } else {
          document.documentElement.classList.add("light");
        }
      }
    };
    mediaQuery.addEventListener("change", handleChange);
    return () => mediaQuery.removeEventListener("change", handleChange);
  }, [theme]);

  async function handleThemeChange(newTheme) {
    setTheme(newTheme);
    await invoke("set_theme", { theme: newTheme });
  }

  async function addPr() {
    try {
      const newList = await invoke("add_item", { url: prUrl });
      setPrList(newList);
    } catch (error) {
      updateErrorMessage(error);
    } finally {
      setPrUrl("");
    }
  }

  async function deletePr(prNumber) {
    try {
      await invoke("delete_pr", { prNumber });
      setPrList((currentList) =>
        currentList.filter((pr) => pr.pr_number !== prNumber)
      );
    } catch (error) {
      updateErrorMessage(error);
    }
  }

  const updateErrorMessage = (message) => {
    setErrorMessage(message);
    setTimeout(() => setErrorMessage(""), 5000);
  };

  useEffect(() => {
    async function initializeState() {
      invoke("has_token").then(setHasToken).catch(console.error);
      invoke("get_pr_list").then(setPrList).catch(console.error);
      invoke("get_refresh_time")
        .then((t) => setRefreshTime(t / 60))
        .catch(console.error);
      invoke("get_show_notification")
        .then(setShowNotification)
        .catch(console.error);
    }
    initializeState();

    const unlistenError = listen("error-event", (event) => {
      updateErrorMessage(event.payload);
    });
    const unlistenPrClosed = listen("pr-closed", (event) => {
      const prNumber = parseInt(event.payload, 10);
      setPrList((currentList) =>
        currentList.map((pr) =>
          pr.pr_number === prNumber ? { ...pr, state: "closed" } : pr
        )
      );
    });

    return () => {
      unlistenError.then((fn) => fn());
      unlistenPrClosed.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    invoke("start_task").catch(console.error);
    return () => {
      invoke("stop_task").catch(console.error);
    };
  }, []);

  const handleSubmitForm = (e) => {
    e.preventDefault();
    const token = e.target.elements["token-input"].value;
    invoke("add_token", { token }).then(() => setHasToken(true));
  };

  const buildUrlFromPr = (pr) =>
    `https://github.com/${pr.owner}/${pr.repo}/pull/${pr.pr_number}`;

  if (!hasToken) {
    return (
      <main className="m-4 text-center bg-gray-200 dark:bg-gray-800 text-black dark:text-white rounded-lg p-4">
        <h1 className="font-bold">Enter your GitHub Token</h1>
        <form className="mt-2" onSubmit={handleSubmitForm}>
          <input
            id="token-input"
            className="w-80 h-12 rounded-lg bg-white dark:bg-gray-700 placeholder:text-center focus:outline-none"
            placeholder="Enter token"
          />
          <button
            className="ml-2 border-2 border-blue-500 rounded-full w-12 h-12 bg-blue-500 text-white"
            type="submit"
          >
            ➕
          </button>
        </form>
        <p className="mt-2 text-sm">
          Click{" "}
          <a
            className="text-blue-500 hover:underline"
            href="https://github.com/settings/tokens/new"
            target="_blank"
            rel="noopener noreferrer"
          >
            here
          </a>{" "}
          to generate a new token.
        </p>
      </main>
    );
  }

  const activeTabStyle = "border-blue-500 text-blue-600 dark:text-blue-400 dark:border-blue-400";
  const inactiveTabStyle = "border-transparent hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300";

  const prListOpen = prList.filter((pr) => pr.state === "open");
  const prListClosed = prList.filter((pr) => pr.state === "closed");

  return (
    <main className="bg-white dark:bg-gray-900 text-black dark:text-white min-h-screen rounded-lg overflow-hidden">
      {errorMessage && (
        <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 absolute w-full dark:bg-red-900 dark:text-red-300 dark:border-red-600" role="alert">
          <p className="font-bold">Error: {errorMessage}</p>
        </div>
      )}
      <div data-tauri-drag-region className="border-b border-gray-200 dark:border-gray-700">
        <ul className="flex flex-wrap -mb-px text-sm font-medium text-center">
          {["Open", "Closed", "Settings"].map((tabName) => (
            <li key={tabName} className="me-2">
              <button
                onClick={() => setActiveTab(tabName.toLowerCase())}
                className={`inline-block p-4 border-b-2 rounded-t-lg ${
                  activeTab === tabName.toLowerCase() ? activeTabStyle : inactiveTabStyle
                }`}
              >
                {tabName}
              </button>
            </li>
          ))}
        </ul>
      </div>

      <div style={{ display: activeTab === "open" ? "block" : "none" }}>
        <form className="flex justify-between m-2" onSubmit={(e) => { e.preventDefault(); addPr(); }}>
          <input
            value={prUrl}
            className="rounded bg-gray-100 dark:bg-gray-700 focus:outline-none w-full mr-2 px-2"
            onChange={(e) => setPrUrl(e.currentTarget.value)}
            placeholder="Pull request link"
          />
          <button className="border-2 border-blue-500 rounded-full w-10 h-10 bg-blue-500 text-white flex-shrink-0" type="submit">
            ➕
          </button>
        </form>
        <div className="relative overflow-x-auto bg-gray-100 dark:bg-gray-800 m-2 rounded-lg">
          <ul className="divide-y divide-gray-200 dark:divide-gray-700">
            {prListOpen.length > 0 ? (
              prListOpen.map((pr) => (
                <li key={pr.pr_number} className="flex items-center justify-between p-2">
                  <div className="flex-grow overflow-hidden whitespace-nowrap">
                    <a href={buildUrlFromPr(pr)} target="_blank" rel="noopener noreferrer" className="hover:underline">
                      {pr.title}
                    </a>
                  </div>
                  <button onClick={() => deletePr(pr.pr_number)} className="text-red-500 hover:text-red-700 font-bold p-1 ml-2 flex-shrink-0">
                    &times;
                  </button>
                </li>
              ))
            ) : (
              <p className="text-center p-4 text-gray-500 dark:text-gray-400">
                No open pull requests
              </p>
            )}
          </ul>
        </div>
      </div>
      
      <div style={{ display: activeTab === "closed" ? "block" : "none" }}>
      <div className="relative overflow-x-auto bg-gray-100 dark:bg-gray-800 m-2 rounded-lg">
          <ul className="divide-y divide-gray-200 dark:divide-gray-700">
            {prListClosed.length > 0 ? (
              prListClosed.map((pr) => (
                <li key={pr.pr_number} className="flex items-center justify-between p-2">
                  <div className="flex-grow overflow-hidden whitespace-nowrap">
                    <a href={buildUrlFromPr(pr)} target="_blank" rel="noopener noreferrer" className="hover:underline">
                      {pr.title}
                    </a>
                  </div>
                  <button onClick={() => deletePr(pr.pr_number)} className="text-red-500 hover:text-red-700 font-bold p-1 ml-2 flex-shrink-0">
                    &times;
                  </button>
                </li>
              ))
            ) : (
              <p className="text-center p-4 text-gray-500 dark:text-gray-400">
                No closed pull requests
              </p>
            )}
          </ul>
        </div>
      </div>

      <div style={{ display: activeTab === "settings" ? "block" : "none" }}>
        <div className="p-4 flex flex-col items-start gap-4">
          <div className="flex items-center justify-start gap-4">
            <label className="text-gray-600 dark:text-gray-300">Theme:</label>
            <div className="relative">
              <select value={theme} onChange={(e) => handleThemeChange(e.target.value)} className="appearance-none rounded bg-gray-100 dark:bg-gray-700 focus:outline-none py-2 px-8">
                <option value="system">System</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
              <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-gray-700 dark:text-gray-300">
                <svg className="fill-current h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
                  <path d="M5.516 7.548c.436-.446 1.043-.481 1.576 0L10 10.405l2.908-2.857c.533-.481 1.14-.446 1.576 0 .436.445.408 1.197 0 1.642l-3.417 3.357c-.27.267-.626.402-.98.402s-.71-.135-.98-.402L5.516 9.19c-.408-.445-.436-1.197 0-1.642z" />
                </svg>
              </div>
            </div>
          </div>
          <form className="flex items-center justify-start gap-4" onSubmit={(e) => { e.preventDefault(); invoke("set_refresh_time", { timeInMinutes: Number(refreshTime) }); }}>
            <label htmlFor="refresh-time-input" className="text-gray-600 dark:text-gray-300">Refresh time (minutes):</label>
            <input
              id="refresh-time-input"
              type="number"
              min="1"
              value={refreshTime}
              className="rounded bg-gray-100 dark:bg-gray-700 focus:outline-none w-20 text-center"
              onChange={(e) => setRefreshTime(e.currentTarget.value)}
            />
            <button className="border-2 border-blue-500 rounded-lg px-4 py-1 bg-blue-500 text-white" type="submit">
              Save
            </button>
          </form>
          <div className="flex items-center justify-start gap-2">
            <label htmlFor="show-notification-input" className="text-gray-600 dark:text-gray-300">Show notifications:</label>
            <button
              id="show-notification-input"
              role="switch"
              aria-checked={showNotification}
              onClick={() => {
                const newShowNotification = !showNotification;
                setShowNotification(newShowNotification);
                invoke("set_show_notification", { show: newShowNotification });
              }}
              className={`${showNotification ? "bg-blue-500" : "bg-gray-200 dark:bg-gray-700"} relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none`}
            >
              <span
                aria-hidden="true"
                className={`${showNotification ? "translate-x-5" : "translate-x-0"} pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out`}
              />
            </button>
          </div>
        </div>
      </div>
    </main>
  );
}

export default App;
