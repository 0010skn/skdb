# ✨💖 SKDB - A Super Duper Cute Database! 💖✨

Hello there, future data adventurer! (づ｡◕‿‿◕｡)づ Welcome to **SKDB**, the most adorable and powerful database you'll ever meet! 🥳 SKDB is here to make your data management tasks as joyful as a walk in a field of fluffy clouds and rainbows! 🌈☁️

This little marvel is designed to be super easy to use, incredibly fast, and oh-so-flexible! Whether you're building a tiny app or a big, complex system, SKDB is your trusty companion! 🚀

---

# 🚀 How to Run SKDB! 🚀

Getting SKDB up and running is as easy as pie! 🥧 Just follow these simple steps, and you'll be querying data in no time! (｡♥‿♥｡)

1.  **Open your magical command line terminal!** ✨
2.  **Type in the special command:**

    ```bash
    cargo run --bin sk-runtime -- -f examples/runtime_tests/main_test.hs "#.department{D101}.p.proj_name"
    ```

3.  **Let's break down this magical spell (command)!** 🪄
    - `cargo run --bin sk-runtime`: This part tells your computer to run our special SKDB program called `sk-runtime`. Think of it as waking up our little data genie! 🧞
    - `--`: This tiny symbol is like a little bridge, separating the command to run the program from the instructions we give to the program itself!
    - `-f examples/runtime_tests/main_test.hs`: The `-f` flag is like saying "Hey SKDB, please look at this file!". And `examples/runtime_tests/main_test.hs` is the path to a file containing some example data and schema. It's like giving our genie a treasure map! 🗺️
    - `"#.department{D101}.p.proj_name"`: This is your actual query! It's like asking our genie a specific question. In this case, you're asking for the project name (`proj_name`) associated with the project details (`p`) of the department (`department`) with the ID `D101`. So clever! 🧐

Isn't that fun and easy? Now you're ready to explore your data! 🎉

---

# 📄 HS Code Examples - Let's See Some Magic! 📄

SKDB uses a super cute and simple way to define your data structures (schemas) and add data! It's all done in `.hs` files. Let's peek at some examples from our [`examples/runtime_tests/main_test.hs`](examples/runtime_tests/main_test.hs:1) file! ૮꒰˶• ༝ •˶꒱ა ♡

**1. Defining a Project Structure (Defining how projects look like!)** 🏗️

```hs
project_structure:
/proj_name/deadline/
Alpha Project,2025-12-31
~
```

- `project_structure:`: This is the name of our table! We're calling it `project_structure`. Cute, right?
- `/proj_name/deadline/`: These are the column names! So, our `project_structure` table will have a `proj_name` (project name) and a `deadline`.
- `Alpha Project,2025-12-31`: This is a row of data! It means we have a project named "Alpha Project" with a deadline of "2025-12-31".
- `~`: This little squiggle means "end of table data here!"

**2. Keeping Track of Active Projects (What are we working on now?)** 📝

```hs
active_projects:
/proj_id::sindex/proj_name/status/deadline/
0,Alpha Project,In Progress,2025-12-31
~
```

- `active_projects:`: Another table, this time for our active projects!
- `/proj_id::sindex/proj_name/status/deadline/`: Look at these columns! We have `proj_id` (project ID, and `::sindex` means it's a special speedy index!), `proj_name`, `status` (like "In Progress"), and `deadline`.
- `0,Alpha Project,In Progress,2025-12-31`: Data for an active project! Project ID 0 is "Alpha Project", it's "In Progress", and due by "2025-12-31".

**3. Listing Our Awesome Departments (Where the magic happens!)** 🏢

```hs
department:
/dept_id::index/dept_name/location/p::project_structure/
D101,Engineering,Building A,(Alpha Project,2025-12-31)
~
```

- `department:`: Our department table!
- `/dept_id::index/dept_name/location/p::project_structure/`: Columns galore! `dept_id` (department ID, `::index` makes it searchable!), `dept_name`, `location`, and `p::project_structure` (this is super cool! `p` is a nested table using the `project_structure` schema we defined earlier! It links department to its project details!).
- `D101,Engineering,Building A,(Alpha Project,2025-12-31)`: The "Engineering" department (ID D101) is in "Building A" and is associated with the "Alpha Project" which has a deadline of "2025-12-31".

See? It's like building with colorful blocks! 🧱 You define how your data looks, and then you fill it with all your important information! SKDB makes it all so simple and fun! 🥰

---

---

# ✨💖 SKDB - 超级无敌可爱的数据库！💖✨

你好呀，未来的数据小探险家！(づ｡◕‿‿◕｡)づ 欢迎来到 **SKDB**，这是你将会遇到的最最可爱、也最最强大的数据库哦！🥳 SKDB 的使命就是让你的数据管理任务变得像在松软的云朵和绚丽的彩虹中漫步一样快乐！🌈☁️

这个小可爱被设计得超级易用、快得不可思议，而且哦哦哦——超级灵活！无论你是在构建一个小小的应用程序，还是一个庞大复杂的系统，SKDB 都是你最值得信赖的小伙伴！🚀

---

# 🚀 如何运行 SKDB 呀！🚀

让 SKDB 跑起来简直比吃小蛋糕还要简单！🍰 只需要跟着这些简单的步骤，你马上就能查询数据啦！(｡♥‿♥｡)

1.  **打开你神奇的命令行终端！** ✨
2.  **输入这条特殊的咒语 (命令)：**

    ```bash
    cargo run --bin sk-runtime -- -f examples/runtime_tests/main_test.hs "#.department{D101}.p.proj_name"
    ```

3.  **让我们来解析一下这条神奇的咒语 (命令) 吧！** 🪄
    - `cargo run --bin sk-runtime`: 这部分告诉你的电脑去运行我们名为 `sk-runtime` 的 SKDB 特别程序。把它想象成唤醒我们的小小数据精灵！🧞
    - `--`: 这个小小的符号就像一座小桥，把运行程序的命令和我们给程序本身的指令分开啦！
    - `-f examples/runtime_tests/main_test.hs`: `-f` 标志就像在说：“嘿，SKDB，请看看这个文件！” 而 `examples/runtime_tests/main_test.hs` 就是包含了一些示例数据和表结构的文件的路径。这就像给了我们的小精灵一张藏宝图！🗺️
    - `"#.department{D101}.p.proj_name"`: 这就是你真正的查询语句啦！就像向我们的小精灵提出了一个具体的问题。在这个例子里，你是在查询 ID 为 `D101` 的部门 (`department`) 其项目详情 (`p`) 中的项目名称 (`proj_name`)。太聪明啦！🧐

是不是又好玩又简单？现在你准备好去探索你的数据啦！🎉

---

# 📄 HS 代码示例 - 一起来看看魔法吧！📄

SKDB 用一种超级可爱和简单的方式来定义你的数据结构 (表结构/schema) 和添加数据哦！这些都是在 `.hs` 文件里完成的。让我们来偷看一下我们 [`examples/runtime_tests/main_test.hs`](examples/runtime_tests/main_test.hs:1) 文件里的一些例子吧！૮꒰˶• ༝ •˶꒱ა ♡

**1. 定义项目结构 (定义项目长什么样子哒！)** 🏗️

```hs
project_structure:
/proj_name/deadline/
Alpha Project,2025-12-31
~
```

- `project_structure:`: 这是我们表的名字！我们叫它 `project_structure`。很可爱，对不对？
- `/proj_name/deadline/`: 这些是列名哦！所以，我们的 `project_structure` 表会有一个 `proj_name` (项目名称) 和一个 `deadline` (截止日期)。
- `Alpha Project,2025-12-31`: 这是一行数据！它表示我们有一个名为 “Alpha Project” 的项目，截止日期是 “2025-12-31”。
- `~`: 这个小小的波浪号意思是“表数据到这里结束啦！”

**2. 记录当前进行中的项目 (我们现在在忙些什么呀？)** 📝

```hs
active_projects:
/proj_id::sindex/proj_name/status/deadline/
0,Alpha Project,In Progress,2025-12-31
~
```

- `active_projects:`: 另一个表，这次是给我们当前进行中的项目哒！
- `/proj_id::sindex/proj_name/status/deadline/`:看看这些列！我们有 `proj_id` (项目 ID，`::sindex` 表示它是一个特殊的快速索引哦！)、`proj_name`、`status` (比如 “In Progress” 表示进行中) 和 `deadline`。
- `0,Alpha Project,In Progress,2025-12-31`: 进行中项目的数据！项目 ID 为 0 的是 “Alpha Project”，它目前状态是 “In Progress”，截止日期是 “2025-12-31”。

**3. 列出我们超棒的部门 (魔法发生的地方！)** 🏢

```hs
department:
/dept_id::index/dept_name/location/p::project_structure/
D101,Engineering,Building A,(Alpha Project,2025-12-31)
~
```

- `department:`: 我们的部门表！
- `/dept_id::index/dept_name/location/p::project_structure/`: 好多列呀！`dept_id` (部门 ID，`::index` 让它可以被搜索到！)、`dept_name`、`location`，还有 `p::project_structure` (这个超酷的！`p` 是一个嵌套表，它使用了我们之前定义的 `project_structure` 结构！它把部门和它的项目详情连接起来啦！)。
- `D101,Engineering,Building A,(Alpha Project,2025-12-31)`: “Engineering” 部门 (ID 是 D101) 在 “Building A”，并且它关联了 “Alpha Project” 项目，该项目的截止日期是 “2025-12-31”。

看吧？就像搭积木一样简单！🧱 你定义好你的数据长什么样，然后用你所有重要的信息把它填满！SKDB 让一切都变得这么简单又有趣！🥰
