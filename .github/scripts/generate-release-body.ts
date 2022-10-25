import { Octokit } from "octokit";
import yargs from "yargs";
import { execSync } from "child_process";
import { readFileSync } from "fs";
import path from "path";

// Typescript 4 will support it natively
type Await<T> = T extends PromiseLike<infer U> ? U : T;
type Commits = Await<ReturnType<Octokit["rest"]["repos"]["compareCommits"]>>["data"]["commits"];

function getCompareLink(packageName: string, previousTag: string, newTag: string) {
  // Previous
  const previousPackage = execSync(
    `git show ${previousTag}:../../Cargo.lock | grep ${packageName}? | head -1 | grep -o '".*"'`
  ).toString();

  const previousCommitTmp = /#([0-9a-f]*)/g.exec(previousPackage);
  if (previousCommitTmp == null) { // regexp didn't match
    return ""
  };
  const previousCommit = previousCommitTmp[1].slice(0, 8);

  const previousRepoTmp = /(https:\/\/.*)\?/g.exec(previousPackage);
  if (previousRepoTmp == null) {
    return ""
  };
  const previousRepo = previousRepoTmp[1];

  // New
  const newPackage = execSync(
    `git show ${newTag}:../../Cargo.lock | grep ${packageName}? | head -1 | grep -o '".*"'`
  ).toString();
  const newCommitTmp = /#([0-9a-f]*)/g.exec(newPackage)
  if (newCommitTmp == null) {
    return ""
  };
  const newCommit = newCommitTmp[1].slice(0, 8);

  const newRepoTmp = /(https:\/\/.*)\?/g.exec(newPackage);
  if (newRepoTmp == null) {
    return ""
  }
  const newRepo = newRepoTmp[1];
  const newRepoOrganization = /github.com\/([^\/]*)/g.exec(newRepo)[1];

  const diffLink =
    previousRepo !== newRepo
      ? `${previousRepo}/compare/${previousCommit}...${newRepoOrganization}:${newCommit}`
      : `${previousRepo}/compare/${previousCommit}...${newCommit}`;

  return diffLink;
}

async function getCommitAndLabels(
  octokit: Octokit,
  owner: string,
  repo: string,
  previousTag: string,
  newTag: string
): Promise<{ prByLabels: any; commits: any[] }> {
  let commits: Commits = [];
  let more = true;
  let page = 0;
  while (more) {
    const compare = await octokit.rest.repos.compareCommitsWithBasehead({
      owner,
      repo,
      basehead: previousTag + "..." + newTag,
      per_page: 200,
      page,
    });
    commits = commits.concat(compare.data.commits);
    more = compare.data.commits.length == 200;
    page++;
  }

  // Determine commits to exclude
  // - commits reverted in the same range
  const excludedCommits: number[] = [];
  const revertedCommits: number[] = [];
  for (let i = commits.length - 1; i >= 0; i--) {
    const commitMessageFirstLine = commits[i].commit.message.split("\n")[0].trim();

    if (revertedCommits[commitMessageFirstLine] != null) {
      excludedCommits.push(i);
      excludedCommits.push(revertedCommits[commitMessageFirstLine]);
    } else {
      const foundRevertedCommitName = commitMessageFirstLine.match(/Revert \"(.*)\"/);
      if (foundRevertedCommitName?.length > 0) {
        revertedCommits[foundRevertedCommitName[1]] = i;
      }
    }
  }

  const prByLabels = {};
  for (let i = 0; i < commits.length; i++) {
    const commitMessageFirstLine = commits[i].commit.message.split("\n")[0].trim();
    if (!excludedCommits.includes(i)) {
      const foundPrsNumbers = commitMessageFirstLine.match(/\(#([0-9]+)\)$/);
      if (foundPrsNumbers && foundPrsNumbers.length > 1) {
        // This will check current repo and if the PR is not found, will try the official repo
        const repos = [
          { owner, repo },
          { owner: "AstarNetwork", repo: "Astar" },
        ];
        for (const { owner, repo } of repos) {
          try {
            const pr = await octokit.rest.pulls.get({
              owner,
              repo,
              pull_number: parseInt(foundPrsNumbers[1]),
            });

            if (pr.data.labels && pr.data.labels.length > 0) {
              for (const label of pr.data.labels) {
                prByLabels[label.name] = prByLabels[label.name] || [];
                prByLabels[label.name].push(pr.data);
              }
            } else {
              prByLabels[""] = prByLabels[""] || [];
              prByLabels[""].push(pr);
            }
            break;
          } catch (e) {
            // PR not found... let's try the other repo
          }
        }
      }
    }
  }
  return {
    prByLabels,
    commits,
  };
}

function getRuntimeInfo(srtoolReportFolder: string, runtimeName: string) {
  const specVersion = execSync(
    `cat ../../runtime/${runtimeName}/src/lib.rs | grep 'spec_version: [0-9]*' | tail -1`
  ).toString();
  return {
    name: runtimeName,
    version: /:\s?([0-9A-z\-]*)/.exec(specVersion)[1],
    srtool: JSON.parse(
      readFileSync(path.join(srtoolReportFolder, `./${runtimeName}-srtool-digest.json`)).toString()
    ),
  };
}

function capitalize(s) {
  return s[0].toUpperCase() + s.slice(1);
}

const CLIENT_CHANGES_LABEL = "client";
const RUNTIME_CHANGES_LABEL = "runtime"
const BREAKING_CHANGES_LABEL = "breaksapi";

async function main() {
  const argv = yargs(process.argv.slice(2))
    .usage("npm run ts-node generate-release-body.ts [args]")
    .version("1.0.0")
    .options({
      from: {
        type: "string",
        describe: "previous tag to retrieve commits from",
        required: true,
      },
      to: {
        type: "string",
        describe: "current tag being drafted",
        required: true,
      },
      owner: {
        type: "string",
        describe: "Repository owner (Ex: AstarNetwork)",
        required: true,
      },
      repo: {
        type: "string",
        describe: "Repository name (Ex: Astar)",
        required: true,
      },
    })
    .demandOption(["from", "to"])
    .help().argv;

  const octokit = new Octokit({
    auth: process.env.GITHUB_TOKEN || undefined,
  });

  const previousTag = argv.from;
  const newTag = argv.to;

  const runtimes = ["shibuya", "shiden", "astar"].map((runtimeName) =>
    getRuntimeInfo(argv["srtool-report-folder"], runtimeName)
  );

  const moduleLinks = ["substrate", "polkadot", "cumulus", "frontier", "astar-frame"].map((repoName) => ({
    name: repoName,
    link: getCompareLink(repoName, previousTag, newTag),
  }));

  const { prByLabels } = await getCommitAndLabels(
    octokit,
    argv.owner,
    argv.repo,
    previousTag,
    newTag
  );

  const clientPRs = prByLabels[CLIENT_CHANGES_LABEL] || [];
  const runtimePRs = prByLabels[RUNTIME_CHANGES_LABEL] || [];
  const emptyLabelPRs = prByLabels[''] || [];

  const printPr = (pr) => {
    if (pr.data.labels.includes(BREAKING_CHANGES_LABEL)) {
        return "⚠️ " + pr.data.title + " (#" + pr.data.number + ")";
    }
    return pr.data.title + " (#" + pr.data.number + ")";
  };

  const template = `
## Description
(Placeholder for release descriptions, please freely write explanations for this release here.)

\*\*Upgrade priority: LOW/MID/HIGH\*\*

${runtimes.length > 0 ? `## Runtimes
${runtimes
  .map(
    (runtime) => `### ${capitalize(runtime.name)}
\`\`\`
✨ spec_version:                ${runtime.version}
🏋 Runtime Size:                ${runtime.srtool.runtimes.compressed.size}
🗜 Compressed:                  ${runtime.srtool.runtimes.compressed.subwasm.compression.compressed ? "Yes" : "No" }
🎁 Metadata version:            ${ runtime.srtool.runtimes.compressed.subwasm.metadata_version }
🗳️ sha256:                      ${runtime.srtool.runtimes.compressed.sha256}
🗳️ blake2-256:                  ${runtime.srtool.runtimes.compressed.blake2_256}
🗳️ proposal (authorizeUpgrade): ${runtime.srtool.runtimes.compressed.subwasm.parachain_authorize_upgrade_hash}
📦 IPFS:                        ${runtime.srtool.runtimes.compressed.subwasm.ipfs_hash}
\`\`\`
`).join(`\n`)}` : ""}

## Build Info
WASM runtime built using \`${runtimes[0]?.srtool.info.rustc}\`

## Changes
### Client
${clientPRs.length > 0 ? `
${clientPRs.map((pr) => `* ${printPr(pr)}`).join("\n")}
` : "None"}
### Runtime
${runtimePRs.length > 0 ? `
${runtimePRs.map((pr) => `* ${printPr(pr)}`).join("\n")}
` : "None"}
### Others
${emptyLabelPRs.length > 0 ? `
${emptyLabelPRs.map((pr) => `* ${printPr(pr)}`).join("\n")}
` : "None"}

## Dependency Changes
Astar: https://github.com/${argv.owner}/${argv.repo}/compare/${previousTag}...${newTag}
${moduleLinks.map((modules) => `${capitalize(modules.name)}: ${modules.link}`).join("\n")}

## Download Links
| Arch |  Link  |
| ----------- | ------- |
|  \`MacOS\` | [Download](https://github.com/AstarNetwork/Astar/releases/download/${newTag}/astar-collator-${newTag}-macOS-x86_64.tar.gz) |
| \`Ubuntu\` | [Download](https://github.com/AstarNetwork/Astar/releases/download/${newTag}/astar-collator-${newTag}-ubuntu-x86_64.tar.gz) |

[<img src="https://www.docker.com/sites/default/files/d8/2019-07/vertical-logo-monochromatic.png" height="200px">](https://hub.docker.com/r/staketechnologies/astar-collator/tags) 
`

  console.log(template);
}

main();
