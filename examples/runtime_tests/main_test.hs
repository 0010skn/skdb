project_structure:
/proj_name/deadline/
Alpha Project,2025-12-31
~
employee:
/emp_id::sindex/emp_name/dept_ref::department/project_ids::string/
0,John Doe,D101,P001 P002
~
active_projects:
/proj_id::sindex/proj_name/status/deadline/
0,Alpha Project,In Progress,2025-12-31
~
department:
/dept_id::index/dept_name/location/p::project_structure/
D101,Engineering,Building A,(Alpha Project,2025-12-31)
D102,Marketing,Building B,(D102,2025-01-01)
D103,HR,Building C,(D103,2025-01-01)
~
