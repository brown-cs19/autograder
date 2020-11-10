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

    // Use late due date, if present.
    let due_date: string;
    if (data.assignment.late_due_date === null) {
        due_date = data.assignment.due_date;
    } else {
        due_date = data.assignment.late_due_date;
    }


    if (Date.now() < Date.parse(due_date)) {
        console.log("true");
    } else {
        console.log("false");
    }
}

main();