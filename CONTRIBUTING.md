# Contributing to waagent-rs

## Hackathon warning (will be removed)

This project is part of a Microsoft Hackathon. The main goal is to have fun and learn some technologies like:

- Git and GitHub workflows
- GitHub actions with local runners
- GitHub actions for testing and releasing code
- Rust programming
- How does waagent work
- How to build packages for distributions and to do cargo builds

## GitHub workflow

It is recommended but not required to be part of the "waagent-rs" GitHub org.

The workflow we will use for this project is going to be:

Step zero: Log in to your github account in the browser. Also get ready a terminal that has git, your favorite dev tools, Rust, and the ssh key you have registered inside of your GitHub user.

- Fork:
  - Start by forking the project. This will create a copy of the waagent-rs under your user.
  - Go to the web page of the [main project](https://github.com/waagent-rs/waagent-rs)
  - Click on the Fork button
- Clone:
  - From inside your Fork, click on the green button that says "Code". Now verify that you are in "Local", and below that you should see another tab that says SSH. Copy the url to the clipboard.
  - In a terminal, paste the url from the previous step, so that it says: `git clone git@github.com:username/waagent-rs.git`, where username is your github username.
  - Now do `cd waagent-rs`
- Branch:
  - Think of the name of the new branch, where its name is going to be related to the work that you are going to do. It is also a good idea to use separate branches if you plan on doing multiple things. For example, one branch for "doc-update" and another branch for "ci-tests". This will make one of the next steps, the **Pull Request** easier to merge to the main project, as it will be clear what part of the code or docs you are modifying.
  - From the terminal write: `git branch newname` and then `git checkout newname`, where newname is just an example word.
- Edit:
  - Using your favorite editor or tools, modify the code, doc, test, etc.
  - Things to keep in mind:
    - If you will be working on multiple things, use the checkout command to switch in between previously created branches.
    - Try to keep the code working always. Even if you are starting to hack something today, and then continue after the weekend, do it in a way that it compiles at all times.
    - Use a "lint" tool to review that you don't have any errors or recommendations pending. A lint tool is similar to a spell checker, but for specific type of files. For this project, using a Markdown lint tool, a TOML file lint tool, and a Rust lint tool might be more than enough.
- Commit:
  - Once your code or docs are ready, give them another look to make sure you didn't keep any "print-debug" code, any TODO commentaries, or maybe code that isn't ready for sending to the main project.
  - Now type `git status` from the top directory of the project. This will list all of the changes you have done.
  - Try not to add all of the directory, as this might include temp files or changes that are not ready to send. Let's add the changed files manually by running something like `git add file1 file2 doc.md test/test1`
  - Now commit those changes. First, think of a message that describe the changes that happened to those files. Now, re-think that message in a way that it would make sense to the *other* developers of the project. Then type: `git commit -m "Added funcionality x and y, updated docs and added a test"`
  - You can continue editing, reviewing and sending more commits. Or you can do everything in one single commit. Some "pro" project will prefer a single commit instead of multiple commits.
- Push:
  - If you go to your GitHub page, you might expect to see your new and shiny code there, right? Well, one of the main features of the tool `git` is that it is designed to have multiple places where the repo can be. So pushing the code is a way to send your local commits to GitHub.
  - Run on the terminal: `git push`
  - This push command should use your ssh keys as a method of authentication. If it asks you for the password, maybe you git a web clone in the second step?
  - The first time you run the git push, the git tool will fail and offer another command instead, that specifies the branch name and talks about an "origin" which will be GitHub. That is fine, copy and paste *that* command.
  - Go back to the GitHub page, and review that the files you added are there, and if you sent multiple commits, they should all be there. Also review that those changes are in the correct branch, and not in "main".
- Pull-request:
  - Called a "PR", the pull request is the last step. By now you should have passed your code through several lint tools, you should have read the functions and docs to verify them, you should have tested compiling and also tested the funcionality you added, either but unit tests or manual tests.
  - It is ok if you have errors in your code, you can fix them by pushing more changes to the branch in your fork, and it is normal to have a code reviewer to ask you to maybe make some extra changes. This is all fine and normal, but again, if you review your code *before* it will be less work for everyone, including you.
  - This step isn't done in the CLI but in the web interface of GitHub. Go to your Fork and the correct **branch** and click **Contribute** and then "Open pull request".
  - In the next page, try to do a very good, multi-paragraph description of what you have done. Put yourself in the shoes of the reviewer, this is the first time they are seeing your code and they need to understand it.
  - If you are working on a bug, you can refer to it by adding the text `waagent-rs/waagent-rs#123` where 123 would be the issue number from the GitHub interface.

Note: There is no need to "squash" pull requests. This is a good thing to do if you pull request contains way to many commits, but not absolutely necessary.

## Glossary

When we refer to 'waagent' that would be the original python version from Microsoft, and waagent-rs is the version that will come out from this project.
