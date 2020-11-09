import sys
import time

# For reading autograder output
import json

# For writing to GSheets
import gspread
from oauth2client.service_account import ServiceAccountCredentials
import pprint

import random
time.sleep(random.randrange(0, 5) * 100)

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
def get_reports(name):
    return list(filter(is_section(name), reports))

# Get grade sheets
spreadsheet = get_spreadsheet()

def get_worksheet(name):
    try:
        sheet = spreadsheet.worksheet(name)
    except:
        try:
            sheet = spreadsheet.add_worksheet(name, NUM_ROWS, NUM_COLS)
            sheet.update([[
                "Assignment Submission ID", 
                "Name", 
                "SID", 
                "Email", 
                "Submission Time"]])
        except:
            sheet = spreadsheet.worksheet(name)

    return sheet


# Add reports in
# updates = []
for section in ["Functionality", "Wheat", "Chaff"]:
    updates = []
    section_reports = get_reports(section)
    reports_map = {report["name"]: report["score"] > 0 
                for report in section_reports}
    sheet_name = f"{assignment_name}_{section}_Autograder"
    sheet = get_worksheet(sheet_name)
    headers, emails = sheet.batch_get(['A1:1', 'D1:D'])
    headers, emails = headers[0], list(zip(*emails))[0][1:]

    new_headers = list(map(lambda report: report["name"], filter(lambda report: report["name"] not in headers, section_reports)))
    # print(f"Debug {section}")
    # print(f"Headers: {headers}")
    # print(f"New headers: {new_headers}")
    # print(f"Reports: {reports_map}")
    if new_headers:
        rang = gspread.utils.rowcol_to_a1(1, len(headers) + 1) + ":1"
        sheet.update(rang, [new_headers])
    headers += new_headers

    report_data = list(map(lambda header: reports_map.get(header, None), headers[5:]))

    for [user_name, user_id, user_email] in user_info:
        user_data = [
            submission_id, 
            user_name, 
            user_id, 
            user_email, 
            submission_time]

        all_data = user_data + report_data

        if user_email in emails:
            submission_row = emails.index(user_email) + 2
            updates.append({
                # 'range': gspread.utils.absolute_range_name(sheet_name, f'A{submission_row}:{submission_row}'),
                'range': f'A{submission_row}:{submission_row}',
                'values': [all_data]})
        else:
            sheet.append_row(all_data)

    sheet.batch_update(updates)




