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
  const [theme, setTheme] = useState("system"); // "light", "dark", "system"

  // Apply theme on component mount and when theme state changes
  useEffect(() => {
    const applyTheme = async () => {
      const savedTheme = await invoke("get_theme").catch(() => "system");
      setTheme(savedTheme);

      if (savedTheme === "dark") {
        document.documentElement.classList.add("dark");
      } else if (savedTheme === "light") {
        document.documentElement.classList.remove("dark");
      } else {
        // System theme
        if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
          document.documentElement.classList.add("dark");
        } else {
          document.documentElement.classList.remove("dark");
        }
      }
    };
    applyTheme();

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = () => {
      if (theme === "system") {
        if (mediaQuery.matches) {
          document.documentElement.classList.add("dark");
        } else {
          document.documentElement.classList.remove("dark");
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
      const prList = await invoke("add_item", { url: prUrl });
      setPrList(prList);
    } catch (error) {
      updateErrorMessage(error);
    } finally {
      setPrUrl("");
    }
  }

  async function deletePr(prNumber) {
    try {
      await invoke("delete_pr", { prNumber: prNumber });
      setPrList((prList) => prList.filter((pr) => pr.pr_number !== prNumber));
    } catch (error) {
      updateErrorMessage(error);
    }
  }
  const updateErrorMessage = (message) => {
    setErrorMessage(message);
    setTimeout(() => {
      setErrorMessage("");
    }, 5000);
  };
  useEffect(() => {
    async function getShowNotification() {
      try {
        const res = await invoke("get_show_notification");
        setShowNotification(res);
      } catch (error) {
        console.error("Error getting show notification:", error);
      }
    }
    async function getRefreshTime() {
      try {
        const res = await invoke("get_refresh_time");
        setRefreshTime(res / 60);
      } catch (error) {
        console.error("Error getting refresh time:", error);
      }
    }
    async function hasToken() {
      try {
        const res = await invoke("has_token");
        setHasToken(res);
      } catch (error) {
        console.error("Error getting token:", error);
      }
    }

    async function getPrList() {
      try {
        const res = await invoke("get_pr_list");
        setPrList(res);
      } catch (error) {
        console.error("Deu erro:", error);
      }
    }
    hasToken();
    getPrList();
    getRefreshTime();
    getShowNotification();
  }, []);

  useEffect(() => {
    const unlisten = listen("error-event", (event) => {
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
      const prNumber = parseInt(event.payload);
      setPrList((prList) =>
        prList.map((pr) => {
          if (pr.pr_number === prNumber) {
            pr.state = "closed";
          }
          return pr;
        }),
      );
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function startTask() {
    try {
      await invoke("start_task");
    } catch (error) {
      console.error(error);
    }
  }

  useEffect(() => {
    startTask();
    return () => {
      stopTask();
    };
  }, []);

  async function stopTask() {
    try {
      await invoke("stop_task");
    } catch (error) {
      console.error(error);
    }
  }

  const handleSubmitForm = (e) => {
    e.preventDefault();
    const token = document.getElementById("token-input").value;
    invoke("add_token", { token });
    setHasToken(true);
  };

  const buildUrlFromPr = (pr) =>
    `https://github.com/${pr.owner}/${pr.repo}/pull/${pr.pr_number}`;

  if (!hasToken) {
    return (
      <main className="m-4 text-center bg-gray-200 dark:bg-gray-800 dark:text-white">
        <h1>Enter your github token</h1>
        <form className="mt-2" onSubmit={handleSubmitForm}>
          <input
            className="w-80 h-12 rounded-lg bg-gray-200 dark:bg-gray-700 dark:text-white placeholder:text-center focus:outline-none"
            id="token-input"
            placeholder="Enter token"
          />
          <button
            className="ml-2 border-2 border-gray-600 dark:border-blue-500 rounded-full w-12 h-12 bg-gray-600 dark:bg-blue-500 text-white"
            type="submit"
          >
            ➕
          </button>
        </form>
        <p className="mt-2">
          click{" "}
          <a
            className="text-gray-500 dark:text-blue-400"
            href="https://github.com/settings/tokens/new"
            target="_blank"
          >
            here
          </a>{" "}
          to generate your token{" "}
        </p>
      </main>
    );
  }

  const activeTabStyle =
    "text-gray-600 hover:text-gray-600 dark:text-gray-300 dark:hover:text-gray-300 border-gray-600 dark:border-gray-300";
  const inactiveTabStyle =
    "dark:border-transparent text-gray-500 hover:text-gray-600 dark:text-gray-400 border-gray-100 hover:border-gray-300 dark:border-gray-700 dark:hover:text-gray-300";

  const prListOpen = prList.filter((pr) => pr.state === "open");
  const prListClosed = prList.filter((pr) => pr.state === "closed");

  return (
    <main className="container bg-white dark:bg-gray-900 text-black dark:text-white min-h-screen">
      {errorMessage && (
        <div
          className="bg-red-100 border border-red-400 w-screen h-14 text-red-700 px-4 py-3 absolute dark:bg-red-900 dark:text-red-300 dark:border-red-600"
          role="alert"
        >
          <p className="font-bold">Error: {errorMessage}</p>
        </div>
      )}
      <div className="border-b border-gray-200 dark:border-gray-700">
        <ul
          className="flex flex-wrap -mb-px text-sm font-medium text-center align-middle"
          id="default-styled-tab"
        >
          <li className="me-2" role="presentation">
            <button
              onClick={() => setActiveTab("active")}
              className={
                activeTab === "active"
                  ? "inline-block p-4 border-b-2 rounded-t-lg " + activeTabStyle
                  : "inline-block p-4 border-b-2 rounded-t-lg " +
                  inactiveTabStyle
              }
            >
              Open
            </button>
          </li>
          <li className="me-2" role="presentation">
            <button
              onClick={() => setActiveTab("not-active")}
              className={
                activeTab === "not-active"
                  ? "inline-block p-4 border-b-2 rounded-t-lg " + activeTabStyle
                  : "inline-block p-4 border-b-2 rounded-t-lg " +
                  inactiveTabStyle
              }
            >
              Closed
            </button>
          </li>
          <li className="me-2" role="presentation">
            <button
              onClick={() => setActiveTab("settings")}
              className={
                activeTab === "settings"
                  ? "inline-block p-4 border-b-2 rounded-t-lg " + activeTabStyle
                  : "inline-block p-4 border-b-2 rounded-t-lg " +
                  inactiveTabStyle
              }
            >
              Settings
            </button>
          </li>
        </ul>
      </div>
      <div style={{ display: activeTab === "active" ? "block" : "none" }}>
        <form
          className="flex justify-between m-2"
          onSubmit={(e) => {
            e.preventDefault();
            addPr();
          }}
        >
          <input
            id="greet-input"
            value={prUrl}
            className="rounded bg-gray-100 dark:bg-gray-700 dark:text-white focus:outline-none w-full mr-2"
            onChange={(e) => setPrUrl(e.currentTarget.value)}
            placeholder="  Pull request link"
          />
          <button
            className="border-2 border-gray-600 dark:border-blue-500 rounded-full w-10 h-10 bg-gray-600 dark:bg-blue-500 text-white"
            type="submit"
          >
            ➕
          </button>
        </form>
        {prListOpen.length > 0 && (
          <div className="relative overflow-x-auto bg-gray-100 dark:bg-gray-800 m-2">
            <ul className="max-w-md space-y-1 p-2 text-gray-500 list-none list-inside overflow-x-hidden dark:text-gray-400">
              {prListOpen.map((pullRequest) => (
                <li
                  key={pullRequest.pr_number}
                  className="flex justify-between items-center"
                >
                  <div
                    className={`text-nowrap overflow-hidden w-34 ${
                      pullRequest.title.length > 50
                        ? "hover:animate-carousel"
                        : ""
                    }`}
                  >
                    <a
                      href={buildUrlFromPr(pullRequest)}
                      target="_blank"
                      className="text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
                    >
                      {pullRequest.title}
                    </a>
                  </div>
                  <button
                    onClick={() => deletePr(pullRequest.pr_number)}
                    className="text-red-500 hover:text-red-700 font-bold p-1"
                  >
                    &times;
                  </button>
                  <hr className="dark:border-gray-600" />
                </li>
              ))}
            </ul>
          </div>
        )}
        {prListOpen.length === 0 && (
          <p className="text-center mt-2 text-gray-600 dark:text-gray-400">
            No open pull requests
          </p>
        )}
      </div>
      <div style={{ display: activeTab === "not-active" ? "block" : "none" }}>
        {prListClosed.length > 0 && (
          <div className="relative overflow-x-auto bg-gray-100 dark:bg-gray-800">
            <ul className="max-w-md space-y-1 text-gray-500 list-none list-inside overflow-x-hidden dark:text-gray-400">
              {prListClosed.map((pullRequest) => (
                <li
                  key={pullRequest.pr_number}
                  className="flex justify-between items-center m-2"
                >
                  <div
                    className={`text-nowrap overflow-hidden w-32 ${
                      pullRequest.title.length > 50
                        ? "hover:animate-carousel"
                        : ""
                    }`}
                  >
                    <a
                      href={buildUrlFromPr(pullRequest)}
                      target="_blank"
                      className="text-gray-600 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
                    >
                      {pullRequest.title}
                    </a>
                  </div>
                  <button
                    onClick={() => deletePr(pullRequest.pr_number)}
                    className="text-red-500 hover:text-red-700 font-bold p-1"
                  >
                    &times;
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
        {prListClosed.length === 0 && (
          <p className="text-center mt-2 text-gray-600 dark:text-gray-400">
            No closed pull requests
          </p>
        )}
      </div>
      <div style={{ display: activeTab === "settings" ? "block" : "none" }}>
        <div className="p-4 flex flex-col items-start gap-4">
          <div className="flex items-center justify-start gap-4">
            <label className="text-gray-600 dark:text-gray-300">
              Theme:
            </label>
            <div className="relative">
              <select
                value={theme}
                onChange={(e) => handleThemeChange(e.target.value)}
                className="appearance-none rounded bg-gray-100 dark:bg-gray-700 dark:text-white focus:outline-none py-2 px-8"
              >
                <option value="system">System</option>
                <option value="light">Light</option>
                <option value="dark">Dark</option>
              </select>
              <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-gray-700 dark:text-gray-300">
                <svg
                  className="fill-current h-4 w-4"
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 20 20"
                >
                  <path d="M5.516 7.548c.436-.446 1.043-.481 1.576 0L10 10.405l2.908-2.857c.533-.481 1.14-.446 1.576 0 .436.445.408 1.197 0 1.642l-3.417 3.357c-.27.267-.626.402-.98.402s-.71-.135-.98-.402L5.516 9.19c-.408-.445-.436-1.197 0-1.642z" />
                </svg>
              </div>
            </div>
          </div>
          <form
            className="flex items-center justify-start gap-4"
            onSubmit={(e) => {
              e.preventDefault();
              invoke("set_refresh_time", {
                timeInMinutes: Number(refreshTime),
              });
            }}
          >
            <label
              htmlFor="refresh-time-input"
              className="text-gray-600 dark:text-gray-300"
            >
              Refresh time (minutes):
            </label>
            <input
              id="refresh-time-input"
              type="number"
              min="1"
              value={refreshTime}
              className="rounded bg-gray-100 dark:bg-gray-700 dark:text-white focus:outline-none w-20 text-center"
              onChange={(e) => setRefreshTime(e.currentTarget.value)}
            />
            <button
              className="border-2 border-gray-600 dark:border-blue-500 rounded-lg px-4 py-1 bg-gray-600 dark:bg-blue-500 text-white"
              type="submit"
            >
              Save
            </button>
          </form>
          <div className="flex items-center justify-start gap-2">
            <label
              htmlFor="show-notification-input"
              className="text-gray-600 dark:text-gray-300"
            >
              Show notifications:
            </label>
            <button
              id="show-notification-input"
              role="switch"
              aria-checked={showNotification}
              onClick={() => {
                const newShowNotification = !showNotification;
                setShowNotification(newShowNotification);
                invoke("set_show_notification", {
                  show: newShowNotification,
                });
              }}
              className={`${
                showNotification ? "bg-blue-500" : "bg-gray-200 dark:bg-gray-700"
              } relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none`}
            >
              <span
                aria-hidden="true"
                className={`${
                  showNotification ? "translate-x-5" : "translate-x-0"
                } pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out`}
              />
            </button>
          </div>
        </div>
      </div>
    </main>
  );
}

export default App;
