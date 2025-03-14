import "@uiw/react-textarea-code-editor/dist.css";

import {
  ArrowDownOnSquareIcon,
  ArrowUpOnSquareIcon,
  Button,
  Input,
  Label,
  PlusCircleIcon,
  PlusIcon,
  XCircleIcon,
} from "@dust-tt/sparkle";
import type {
  DatasetEntry,
  DatasetSchema,
  DatasetType,
  DatasetViewType,
} from "@dust-tt/types";
import dynamic from "next/dynamic";
import { useEffect, useRef, useState } from "react";
import TextareaAutosize from "react-textarea-autosize";

import type { DatasetDataType } from "@app/lib/datasets";
import { checkDatasetData, DATASET_DATA_TYPES } from "@app/lib/datasets";
import { getDatasetTypes, getValueType } from "@app/lib/datasets";
import { MODELS_STRING_MAX_LENGTH } from "@app/lib/utils";
import { classNames } from "@app/lib/utils";

const CodeEditor = dynamic(
  () => import("@uiw/react-textarea-code-editor").then((mod) => mod.default),
  { ssr: false }
);

const defaultData = [
  {
    companyName: "Google",
    employees: 100000,
    headquarters: "Mountain View, CA, USA",
  },
  {
    companyName: "Facebook",
    employees: 50000,
    headquarters: "Menlo Park, CA, USA",
  },
  {
    companyName: "Amazon",
    employees: 800000,
    headquarters: "Seattle, WA, USA",
  },
  {
    companyName: "Apple",
    employees: 150000,
    headquarters: "Cupertino, CA, USA",
  },
  {
    companyName: "Microsoft",
    employees: 150000,
    headquarters: "Redmond, WA, USA",
  },
  {
    companyName: "Netflix",
    employees: 10000,
    headquarters: "Los Gatos, CA, USA",
  },
  {
    companyName: "Twitter",
    employees: 5000,
    headquarters: "San Francisco, CA, USA",
  },
  {
    companyName: "Uber",
    employees: 20000,
    headquarters: "San Francisco, CA, USA",
  },
  {
    companyName: "Samsung",
    employees: 287439,
    headquarters: "Seoul, South Korea",
  },
  {
    companyName: "Toyota",
    employees: 359542,
    headquarters: "Toyota City, Japan",
  },
  {
    companyName: "Siemens",
    employees: 293000,
    headquarters: "Munich, Germany",
  },
  {
    companyName: "Nestlé",
    employees: 273000,
    headquarters: "Vevey, Switzerland",
  },
  {
    companyName: "Unilever",
    employees: 149000,
    headquarters: "London, United Kingdom",
  },
  {
    companyName: "Alibaba",
    employees: 117600,
    headquarters: "Hangzhou, China",
  },
  {
    companyName: "Tata Consultancy Services",
    employees: 448464,
    headquarters: "Mumbai, India",
  },
  {
    companyName: "SAP",
    employees: 102430,
    headquarters: "Walldorf, Germany",
  },
  {
    companyName: "Roche",
    employees: 100920,
    headquarters: "Basel, Switzerland",
  },
  {
    companyName: "Sony",
    employees: 109700,
    headquarters: "Tokyo, Japan",
  },
  {
    companyName: "BHP",
    employees: 33000,
    headquarters: "Melbourne, Australia",
  },
  {
    companyName: "Volkswagen",
    employees: 302554,
    headquarters: "Wolfsburg, Germany",
  },
  {
    companyName: "BP",
    employees: 70000,
    headquarters: "London, United Kingdom",
  },
  {
    companyName: "ING Group",
    employees: 57000,
    headquarters: "Amsterdam, Netherlands",
  },
  {
    companyName: "Philips",
    employees: 77000,
    headquarters: "Amsterdam, Netherlands",
  },
  {
    companyName: "Daimler",
    employees: 288481,
    headquarters: "Stuttgart, Germany",
  },
  {
    companyName: "AstraZeneca",
    employees: 70000,
    headquarters: "Cambridge, United Kingdom",
  },
];

const genDefaultDataset = () => {
  const shuffled = defaultData.map((a) => a);
  shuffled.sort(() => 0.5 - Math.random());
  return shuffled.slice(0, 3);
};

export default function DatasetView({
  readOnly,
  datasets,
  dataset,
  schema,
  onUpdate,
  nameDisabled,
  viewType,
}: {
  readOnly: boolean;
  datasets: DatasetType[];
  dataset: DatasetType | null;
  schema: DatasetSchema | null;
  onUpdate: (
    initializing: boolean,
    valid: boolean,
    dataset: DatasetType,
    schema: DatasetSchema
  ) => void;
  nameDisabled: boolean;
  viewType: DatasetViewType;
}) {
  const fileInputRef = useRef<HTMLInputElement>(null);

  if (!dataset) {
    dataset = {
      name: "",
      description: "",
      data: genDefaultDataset(),
    };
  }

  const [datasetName, setDatasetName] = useState(dataset.name);
  const [datasetNameError, setDatasetNameError] = useState("");
  const [datasetDescription, setDatasetDescription] = useState(
    dataset.description
  );
  const [datasetData, setDatasetData] = useState(dataset.data || []);
  const [datasetKeys, setDatasetKeys] = useState(
    checkDatasetData({ data: datasetData })
  );
  const [datasetKeyDescriptions, setDatasetKeyDescriptions] = useState<
    string[]
  >(
    datasetKeys.map((k) => schema?.find((s) => s.key === k)?.description || "")
  );

  const [datasetTypes, setDatasetTypes] = useState<DatasetDataType[]>([]);
  const [datasetInitializing, setDatasetInitializing] = useState(true);

  const datasetNameValidation = () => {
    let valid = true;

    let exists = false;
    datasets.forEach((d) => {
      if (d.name == datasetName && d.name != dataset?.name) {
        exists = true;
      }
    });
    if (exists) {
      setDatasetNameError("A dataset with the same name already exists");
      valid = false;
    } else if (datasetName.length == 0) {
      setDatasetNameError("");
      valid = false;
      // eslint-disable-next-line no-useless-escape
    } else if (!datasetName.match(/^[a-zA-Z0-9\._\-]+$/)) {
      setDatasetNameError(
        "Dataset name must only contain letters, numbers, and the characters `._-`"
      );
      valid = false;
    } else {
      setDatasetNameError("");
    }

    return valid;
  };

  function isTypeValidForDataset(
    enteredType: DatasetDataType,
    desiredType: DatasetDataType
  ): boolean {
    if (desiredType === "string") {
      return true;
    }
    return enteredType === desiredType;
  }

  const datasetTypesValidation = () => {
    // Initial inference of types
    if (datasetTypes.length == 0) {
      inferDatasetTypes();
    }

    // Check that all types are valid
    return datasetData.every((d) =>
      datasetKeys.every((k) =>
        isTypeValidForDataset(
          getValueType(d[k]),
          datasetTypes[datasetKeys.indexOf(k)]
        )
      )
    );
  };

  // Export the dataset with correct types (post-editing and validation)
  const exportDataset = () => {
    const finalDataset = [] as any[];

    datasetData.map((d, i) => {
      const entry = {} as any;
      datasetKeys.map((k) => {
        entry[k] = datasetData[i][k];
        const type = datasetTypes[datasetKeys.indexOf(k)];
        try {
          // Save objects, numbers, and booleans with their proper types
          if (type !== "string") {
            entry[k] = JSON.parse(entry[k]);
          }
        } catch (err) {
          // no-op
        }
      });
      finalDataset.push(entry);
    });

    return finalDataset;
  };

  const inferDatasetTypes = () => {
    if (datasetData.length > 0) {
      setDatasetTypes(getDatasetTypes(datasetKeys, datasetData[0]));
      if (datasetInitializing) {
        setTimeout(() => {
          setDatasetInitializing(false);
        }, 10);
      }
    }
  };

  const inferSchema = (): DatasetSchema => {
    if (datasetData.length > 0) {
      const types = getDatasetTypes(datasetKeys, datasetData[0]);
      return types.map((t, i) => {
        return {
          key: datasetKeys[i],
          type: t,
          description: datasetKeyDescriptions[i] || null,
        };
      });
    } else {
      return [];
    }
  };

  const handleKeyUpdate = (i: number, newKey: string) => {
    const oldKey = datasetKeys[i];
    const data = datasetData.map((d) => {
      d[newKey] = d[oldKey];
      delete d[oldKey];
      return d;
    });
    const keys = datasetKeys.map((k, j) => {
      if (i == j) {
        return newKey;
      }
      return k;
    });
    setDatasetData(data);
    setDatasetKeys(keys);
  };

  const newKey = () => {
    const base = "new_key";
    let idx = 0;
    for (let i = 0; i < datasetKeys.length; i++) {
      if (`${base}_${idx}` == datasetKeys[i]) {
        idx += 1;
        i = 0;
      }
    }
    return `${base}_${idx}`;
  };

  const handleNewKey = (i: number) => {
    const keys = datasetKeys.map((k) => k);
    const n = newKey();
    keys.splice(i + 1, 0, newKey());

    const data = datasetData.map((d) => {
      d[n] = "";
      return d;
    });
    setDatasetData(data);
    setDatasetKeys(keys);

    const types = datasetTypes;
    types.splice(i + 1, 0, "string");
    setDatasetTypes(types);

    const descriptions = datasetKeyDescriptions;
    descriptions.splice(i + 1, 0, "");
    setDatasetKeyDescriptions(descriptions);
  };

  const handleDeleteKey = (i: number) => {
    const data = datasetData.map((d) => {
      delete d[datasetKeys[i]];
      return d;
    });

    setDatasetData(data);

    const keys = datasetKeys.map((k) => k);
    keys.splice(i, 1);
    setDatasetKeys(keys);

    const types = datasetTypes;
    types.splice(i, 1);
    setDatasetTypes(types);

    const descriptions = datasetKeyDescriptions.map((d) => d);
    descriptions.splice(i, 1);
    setDatasetKeyDescriptions(descriptions);
  };

  const handleKeyDescriptionChange = (i: number, value: any) => {
    const descriptions = [...datasetKeyDescriptions];
    descriptions[i] = value;
    setDatasetKeyDescriptions(descriptions);
  };

  const handleValueChange = (i: number, k: string, value: any) => {
    const data = datasetData.map((d, j) => {
      if (i == j) {
        d[k] = value;
      }
      return d;
    });
    setDatasetData(data);
  };

  const handleNewEntry = (i: number) => {
    const data = datasetData.map((d) => {
      return d;
    });
    const entry = {} as DatasetEntry;
    datasetKeys.forEach((k) => {
      entry[k] = "";
    });
    data.splice(i + 1, 0, entry);
    setDatasetData(data);
  };

  const handleDeleteEntry = (i: number) => {
    const data = datasetData.map((d) => {
      return d;
    });
    data.splice(i, 1);
    setDatasetData(data);
  };

  const handleFileLoaded = (e: any) => {
    const content = e.target.result;
    let data = [];
    try {
      data = content
        .split("\n")
        .filter((l: string) => {
          return l.length > 0;
        })
        .map((l: string, i: number) => {
          try {
            return JSON.parse(l);
          } catch (e: any) {
            e.line = i;
            throw e;
          }
        });
    } catch (e: any) {
      window.alert(`Error parsing JSONL line ${e.line}: ${e}`);
      return;
    }
    if (data.length > 256) {
      window.alert("Dataset size is currently limited to 256 entries");
      return;
    }
    let keys = [] as string[];
    try {
      keys = checkDatasetData({
        data,
      });
    } catch (e) {
      window.alert(`${e}`);
    }

    setDatasetKeys(keys);
    setDatasetKeyDescriptions(keys.map(() => ""));
    setDatasetData(data);
    setDatasetTypes([]);

    onUpdate(
      datasetInitializing,
      datasetTypesValidation(),
      {
        name: datasetName.slice(0, MODELS_STRING_MAX_LENGTH),
        description: (datasetDescription || "").slice(
          0,
          MODELS_STRING_MAX_LENGTH
        ),
        data: datasetData,
      },
      inferSchema()
    );
  };

  const handleFileUpload = (file: File) => {
    if (file.size > 1024 * 512) {
      window.alert("JSONL upload size is currently limited to 512KB");
      return;
    }
    const fileData = new FileReader();
    fileData.onloadend = handleFileLoaded;
    fileData.readAsText(file);
  };

  useEffect(() => {
    // Validate the dataset types and dataset name
    const valid = datasetTypesValidation() && datasetNameValidation();

    if (onUpdate) {
      // TODO(spolu): Optimize, as it might not be great to send the entire data on each update.
      onUpdate(
        datasetInitializing,
        valid,
        {
          name: datasetName.slice(0, MODELS_STRING_MAX_LENGTH),
          description: (datasetDescription || "").slice(
            0,
            MODELS_STRING_MAX_LENGTH
          ),
          data: exportDataset(),
        },
        inferSchema()
      );
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    datasetName,
    datasetDescription,
    datasetData,
    datasetKeys,
    datasetKeyDescriptions,
    datasetTypes,
  ]);

  const theme = localStorage.getItem("theme");

  return (
    <div>
      <div className="mt-2 grid gap-x-4 gap-y-4 sm:grid-cols-5">
        {viewType == "full" && (
          <>
            <div className="sm:col-span-1">
              <Label>Dataset Name</Label>
              <Input
                disabled={readOnly || nameDisabled}
                type="text"
                name="name"
                id="datasetName"
                value={datasetName}
                onChange={(e) => setDatasetName(e.target.value)}
                message={datasetNameError}
                messageStatus={datasetNameError ? "error" : "default"}
              />
            </div>

            <div className="sm:col-span-4">
              <Label>Description</Label>
              <Input
                disabled={readOnly}
                type="text"
                name="description"
                id="datasetDescription"
                className="w-full"
                value={datasetDescription || ""}
                onChange={(e) => setDatasetDescription(e.target.value)}
                message="Optional"
                messageStatus="default"
              />
            </div>

            <div className="mt-4 sm:col-span-5">
              <Label>Schema</Label>
              {!readOnly ? (
                <p className="mt-2 text-sm text-foreground dark:text-foreground-night">
                  Set the properties and types to ensure your dataset is valid
                  when you update it. The properties descriptions are used to
                  generate the inputs to your app when run from an Agent.
                </p>
              ) : null}
            </div>

            <div className="sm:col-span-5">
              <div className="space-y-2">
                {datasetKeys.map((k, j) => (
                  <div key={j} className="grid sm:grid-cols-10">
                    <div className="sm:col-span-3">
                      <div className="group flex items-center bg-slate-300 dark:bg-slate-300-night">
                        <div className="flex flex-1">
                          <Input
                            readOnly={readOnly}
                            value={k}
                            onChange={(e) => {
                              handleKeyUpdate(j, e.target.value);
                            }}
                            className="border-0 bg-transparent focus:border-0 focus:ring-0 dark:bg-transparent"
                          />
                        </div>
                        {!readOnly ? (
                          <>
                            {datasetKeys.length > 1 ? (
                              <div className="flex w-4 flex-initial">
                                <XCircleIcon
                                  className="hidden h-4 w-4 cursor-pointer text-gray-400 hover:text-red-500 group-hover:block"
                                  onClick={() => {
                                    handleDeleteKey(j);
                                  }}
                                />
                              </div>
                            ) : null}
                            <div className="mr-2 flex w-4 flex-initial">
                              <PlusCircleIcon
                                className="hidden h-4 w-4 cursor-pointer text-gray-400 hover:text-emerald-500 group-hover:block"
                                onClick={() => {
                                  handleNewKey(j);
                                }}
                              />
                            </div>
                          </>
                        ) : null}
                      </div>
                    </div>
                    <div className="bg-slate-100 dark:bg-slate-100-night sm:col-span-7">
                      {readOnly ? (
                        <span className="block cursor-pointer whitespace-nowrap px-4 py-2 text-sm text-gray-700">
                          {datasetTypes[j] ? datasetTypes[j] : "string"}
                        </span>
                      ) : (
                        <div className="inline-flex" role="group">
                          {DATASET_DATA_TYPES.map((type) => (
                            <Button
                              key={type}
                              type="button"
                              label={type == "json" ? "JSON" : type}
                              disabled={readOnly}
                              onClick={() => {
                                const types = [...datasetTypes];
                                types[j] = type;
                                setDatasetTypes(types);
                              }}
                              variant={
                                datasetTypes && datasetTypes[j] == type
                                  ? "outline"
                                  : "ghost"
                              }
                            />
                          ))}
                        </div>
                      )}
                    </div>
                    <div className="bg-slate-100 dark:bg-slate-100-night sm:col-span-10">
                      <TextareaAutosize
                        minRows={1}
                        className={classNames(
                          "font-mono w-full resize-none border-0 bg-transparent px-1 py-0 text-[13px] font-normal italic placeholder-gray-400 ring-0 focus:ring-0",
                          readOnly
                            ? "text-gray-500"
                            : "text-gray-700 dark:text-gray-700-night"
                        )}
                        readOnly={readOnly}
                        placeholder="Property description"
                        value={datasetKeyDescriptions[j] || ""}
                        onChange={(e) => {
                          handleKeyDescriptionChange(j, e.target.value);
                        }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}

        <div className="mt-4 sm:col-span-5">
          {viewType == "block" ? (
            <p className="text-sm text-gray-500">
              <strong>
                Input data for test-running your app using the 'RUN' button.
              </strong>
              <br />
              Running this app from the API or an agent will use the input data
              provided at runtime.
            </p>
          ) : (
            <>
              <Label>Data</Label>
              {!readOnly ? (
                <p className="mt-2 text-sm text-foreground dark:text-foreground-night">
                  Add and edit your dataset entries below. You can insert or
                  remove entries using buttons on the right.
                </p>
              ) : null}
            </>
          )}
          <div className="mt-4 w-full leading-4">
            <div className="">
              <ul className="space-y-2">
                {datasetData.map((d, i) => (
                  <li key={i} className="space-y-[1px]">
                    {datasetKeys.map((k, j) => (
                      <div key={j} className="grid grid-cols-10">
                        <div className="col-span-3">
                          <div className="group flex items-center bg-slate-300 dark:bg-slate-300-night">
                            <Input
                              readOnly={true}
                              value={k}
                              className={classNames(
                                "border-0 dark:border-0",
                                "bg-transparent dark:bg-transparent",
                                "focus:border-0 dark:focus:border-0",
                                "focus:ring-0 dark:focus:ring-0"
                              )}
                            />
                          </div>
                        </div>
                        <div
                          className={classNames(
                            "font-mono col-span-7 inline-grid resize-none space-y-0 border bg-slate-100 px-0 py-0 text-[13px] dark:bg-slate-100-night",
                            d[k] === "" ||
                              !datasetTypes[datasetKeys.indexOf(k)] ||
                              isTypeValidForDataset(
                                getValueType(d[k]),
                                datasetTypes[datasetKeys.indexOf(k)]
                              )
                              ? "border-slate-100 dark:border-slate-100-night"
                              : "border-red-500"
                          )}
                        >
                          {datasetTypes[datasetKeys.indexOf(k)] === "json" ? (
                            <CodeEditor
                              data-color-mode={
                                theme === "dark" ? "dark" : "light"
                              }
                              readOnly={readOnly}
                              value={
                                typeof d[k] === "string"
                                  ? d[k]
                                  : JSON.stringify(d[k], null, 2)
                              }
                              language="json"
                              onChange={(e) => {
                                handleValueChange(i, k, e.target.value);
                              }}
                              padding={4}
                              className="rounded-lg bg-slate-100 dark:bg-slate-100-night"
                              style={{
                                fontSize: 13,
                                fontFamily:
                                  "ui-monospace, SFMono-Regular, SF Mono, Consolas, Liberation Mono, Menlo, monospace",
                              }}
                            />
                          ) : (
                            <TextareaAutosize
                              minRows={1}
                              className={classNames(
                                "font-mono w-full resize-none border-0 bg-transparent px-1 py-0 text-[13px] font-normal ring-0 focus:ring-0",
                                readOnly
                                  ? "text-gray-500"
                                  : "text-gray-700 dark:text-gray-600"
                              )}
                              readOnly={readOnly}
                              value={d[k]}
                              onChange={(e) => {
                                handleValueChange(i, k, e.target.value);
                              }}
                            />
                          )}
                        </div>
                      </div>
                    ))}
                    {!readOnly ? (
                      <div className="flex items-center justify-end text-xs">
                        {datasetData.length > 1 ? (
                          <div className="flex-initial">
                            <XCircleIcon
                              className="h-4 w-4 cursor-pointer text-gray-300 hover:text-red-500 dark:text-gray-300-night"
                              onClick={() => {
                                handleDeleteEntry(i);
                              }}
                            />
                          </div>
                        ) : null}
                        <div className="flex-initial">
                          <PlusCircleIcon
                            className="h-5 w-5 cursor-pointer text-gray-300 hover:text-emerald-500 dark:text-gray-300-night"
                            onClick={() => {
                              handleNewEntry(i);
                            }}
                          />
                        </div>
                      </div>
                    ) : null}
                  </li>
                ))}
              </ul>
            </div>
            {viewType == "full" && (
              <div className="mt-6 flex flex-row">
                {!readOnly ? (
                  <Button
                    variant="outline"
                    onClick={() => {
                      handleNewEntry(datasetData.length - 1);
                    }}
                    icon={PlusIcon}
                    label="New Entry"
                  />
                ) : null}
                <div className="flex-1"></div>
                <div className="ml-2 flex-initial">
                  <Button
                    variant="ghost"
                    onClick={() => {
                      const dataStr =
                        "data:text/jsonl;charset=utf-8," +
                        encodeURIComponent(
                          exportDataset()
                            .map((d) => JSON.stringify(d))
                            .join("\n")
                        );
                      const downloadAnchorNode = document.createElement("a");
                      downloadAnchorNode.setAttribute("href", dataStr);
                      downloadAnchorNode.setAttribute(
                        "download",
                        `dataset-${dataset?.name}.jsonl`
                      );
                      document.body.appendChild(downloadAnchorNode); // required for firefox
                      downloadAnchorNode.click();
                      downloadAnchorNode.remove();
                    }}
                    icon={ArrowDownOnSquareIcon}
                    label="Download"
                  />
                </div>
                <div className="ml-2 flex-initial">
                  <Input
                    className="hidden"
                    type="file"
                    accept=".jsonl"
                    ref={fileInputRef}
                    onChange={(e) => {
                      if (e.target.files && e.target.files.length > 0) {
                        handleFileUpload(e.target.files[0]);
                      }
                    }}
                  />
                  {!readOnly ? (
                    <Button
                      variant="ghost"
                      onClick={() => {
                        if (fileInputRef.current) {
                          fileInputRef.current.click();
                        }
                      }}
                      icon={ArrowUpOnSquareIcon}
                      label="JSONL"
                    />
                  ) : null}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
