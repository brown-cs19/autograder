function parse_command_line(): string {
    let args: string[] = process.argv.slice(2);

    if (args.length != 1) {
        throw("Usage: <meta_data_file>");
    }

    return args[0];
}

function load_file(path: string) {
    let fs = require('fs');
    let contents: string = fs.readFileSync(path);
    return JSON.parse(contents);
}

function main() {
    let meta_data_file: string = parse_command_line();
    let data = load_file(meta_data_file);
    if (Date.now() < Date.parse(data.assignment.due_date)) {
        console.log("examplar");
    } else {
        console.log("master");
    }
}

main();