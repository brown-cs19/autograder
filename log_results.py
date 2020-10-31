import sys

# For reading autograder output
import json

# For writing to GSheets
import gspread
from oauth2client.service_account import ServiceAccountCredentials
import pprint

(NUM_ROWS, NUM_COLS) = (200, 100)

args = [
        "<path/to/metadata>", 
        "<path/to/autograder/results>",
        "<assignment_name>",
        "<path/to/credentials>",
        "<google_sheet_id>"
    ]
if len(sys.argv) != 1 + len(args):
    raise Exception("Use: log_results.py " + " ".join(args))
(_, metadata_path, results_path, assignment_name, credentials, sheet_id) = sys.argv

def get_spreadsheet():
    #Authorize the API
    scope = [
        'https://www.googleapis.com/auth/drive',
        'https://www.googleapis.com/auth/drive.file'
        ]
    file_name = credentials
    creds = ServiceAccountCredentials.from_json_keyfile_name(file_name, scope)
    client = gspread.authorize(creds)

    #Fetch the sheet
    return client.open_by_key(sheet_id)

def get_json(path):
    with open(path, "r") as f:
        return json.load(f)

# Get submssion information
submission = get_json(metadata_path)
assignment = submission["assignment"]["title"]

submission_id = submission["id"]

user_info = []
for user in submission["users"]:
    user_info.append([user["name"], user["sid"], user["email"]])

submission_time = submission["created_at"]

# Get autograder results
results = get_json(results_path)

is_individual_report = lambda report: report["extra_data"]["type"] == "Individual"
reports = list(filter(is_individual_report, results["tests"]))

is_section = lambda section: lambda report: report["extra_data"]["section"] == section
functionality_reports = list(filter(is_section("Functionality"), reports))
wheat_reports = list(filter(is_section("Wheat"), reports))
chaff_reports = list(filter(is_section("Chaff"), reports))

# Get grade sheets
spreadsheet = get_spreadsheet()

def get_worksheet(name):
    try:
        sheet = spreadsheet.worksheet(name)
    except:
        sheet = spreadsheet.add_worksheet(name, NUM_ROWS, NUM_COLS)

    sheet.update([[
        "Assignment Submission ID", 
        "Name", 
        "SID", 
        "Email", 
        "Submission Time"]])

    return sheet

functionality_sheet = get_worksheet(assignment_name + "_Functionality_Autograder")
wheat_sheet = get_worksheet(assignment_name + "_Wheat_Autograder")
chaff_sheet = get_worksheet(assignment_name + "_Chaff_Autograder")

# Add reports in
def get_row(sheet, user_email):
    try:
        cell = sheet.find(user_email)
        return cell.row
    except:
        email_col = sheet.col_values(4)
        return len(email_col) + 1

def get_column(sheet, header):
    try:
        cell = sheet.find(header)
        return cell.col
    except:
        first_row = sheet.row_values(1)
        col = len(first_row) + 1
        sheet.update_cell(1, col, header)
        return col

for [user_name, user_id, user_email] in user_info:
    sheet_and_reports = [
            [functionality_sheet, functionality_reports],
            [wheat_sheet, wheat_reports],
            [chaff_sheet, chaff_reports]
        ]
    for [sheet, reports] in sheet_and_reports:
        submission_row = get_row(sheet, user_email)
        sheet.update(f'A{submission_row}:E{submission_row}', [[
            submission_id, 
            user_name, 
            user_id, 
            user_email, 
            submission_time]])

        for report in reports:
            report_col = get_column(sheet, report["name"])
            sheet.update_cell(submission_row, report_col, report["score"] > 0)


