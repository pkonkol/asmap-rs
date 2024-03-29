//! this module provides stanford asdb categories autogenerated from NAICSlite.csv
//! The data should not change but in case so it's necessary to regenerate the datastructure
//! and paste it here

pub const CATEGORIES: &[(&str, &[&str])] = &[
    ("Computer and Information Technology", &["Internet Service Provider (ISP)","Phone Provider","Hosting, Cloud Provider, Data Center, Server Colocation","Computer and Network Security","Software Development","Technology Consulting Services","Satellite Communication","Search","Internet Exchange Point (IXP)","Other",]),
    ("Media, Publishing, and Broadcasting", &["Online Music and Video Streaming Services","Online Informational Content","Print Media (Newspapers, Magazines, Books)","Music and Video Industry","Radio and Television Providers","Other",]),
    ("Finance and Insurance", &["Banks, Credit Card Companies, Mortgage Providers","Insurance Carriers and Agencies","Accountants, Tax Preparers, Payroll Services","Investment, Portfolio Management, Pensions and Funds","Other",]),
    ("Education and Research", &["Elementary and Secondary Schools","Colleges, Universities, and Professional Schools","Other Schools, Instruction, and Exam Preparation (Trade Schools, Art Schools, Driving Instruction, etc.)","Research and Development Organizations","Education Software","Other",]),
    ("Service", &["Law, Business, and Consulting Services","Buildings, Repair, Maintenance (Pest Control, Landscaping, Cleaning, Locksmiths, Car Washes, etc)","Personal Care and Lifestyle (Barber Shops, Nail Salons, Diet Centers, Laundry, etc)","Social Assistance (Temporary Shelters, Emergency Relief, Child Day Care, etc)","Other",]),
    ("Agriculture, Mining, and Refineries (Farming, Greenhouses, Mining, Forestry, and Animal Farming)", &[]),
    ("Community Groups and Nonprofits", &["Churches and Religious Organizations","Human Rights and Social Advocacy (Human Rights, Environment and Wildlife Conservation, Other)","Other",]),
    ("Construction and Real Estate", &["Buildings (Residential or Commercial)","Civil Engineering Construction (Utility Lines, Roads and Bridges)","Real Estate (Residential and/or Commercial)","Other",]),
    ("Museums, Libraries, and Entertainment", &["Libraries and Archives","Recreation, Sports, and Performing Arts","Museums, Historical Sites, Zoos, Nature Parks","Casinos and Gambling","Tours and Sightseeing","Other",]),
    ("Utilities (Excluding Internet Service)", &["Electric Power Generation, Transmission, Distribution","Natural Gas Distribution","Water Supply and Irrigation","Sewage Treatment","Steam and Air-Conditioning Supply","Other",]),
    ("Health Care Services", &["Hospitals and Medical Centers","Medical Laboratories and Diagnostic Centers","Nursing, Residential Care Facilities, Assisted Living, and Home Health Care","Other",]),
    ("Travel and Accommodation", &["Air Travel","Railroad Travel","Water Travel","Hotels, Motels, Inns, Other Traveler Accommodation","Recreational Vehicle Parks and Campgrounds","Boarding Houses, Dormitories, Workers’ Camps","Food Services and Drinking Places","Other",]),
    ("Freight, Shipment, and Postal Services", &["Postal Services and Couriers","Air Transportation","Railroad Transportation","Water Transportation","Trucking","Space, Satellites","Passenger Transit (Car, Bus, Taxi, Subway)","Other",]),
    ("Government and Public Administration", &["Military, Defense, National Security, and International Affairs","Law Enforcement, Public Safety, and Justice","Government and Regulatory Agencies, Administrations, Departments, and Services",]),
    ("Retail Stores, Wholesale, and E-commerce Sites", &["Food, Grocery, Beverages","Clothing, Fashion, Luggage","Other",]),
    ("Manufacturing", &["Automotive and Transportation","Food, Beverage, and Tobacco","Clothing and Textiles","Machinery","Chemical and Pharmaceutical Manufacturing","Electronics and Computer Components","Other",]),
    ("Other", &["Individually Owned",]),
    ("Unknown", &[]),
];
